//! Graph/message/audit mutations share the existing single writer and transaction.
use super::*;
use crate::{coordination::{Command,Completion,Payload,Observation,OBSERVATION_MEDIA},scheduler::*};
pub(super) fn exists(db:&Connection,run:RunId)->Result<bool>{Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM coordination WHERE run_id=?1)",[run.0.to_string()],|r|r.get(0))?)}
pub(super) fn graph(db:&Connection,run:RunId)->Result<Graph>{let (document,revision):(String,i64)=db.query_row("SELECT document,revision FROM coordination WHERE run_id=?1",[run.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?;if document.len()>512*1024{return Err(Error::Unsupported);}let graph:Graph=serde_json::from_str(&document).map_err(|_|Error::State)?;graph.plan.validate()?;if sql_integer(graph.revision)?!=revision{return Err(Error::State);}Ok(graph)}
fn actor(graph:&Graph,id:ActorId)->bool{graph.plan.agents.iter().any(|a|a.id==id)}
fn now()->Result<u64>{SystemTime::now().duration_since(UNIX_EPOCH).map(|v|v.as_secs()).map_err(|_|Error::State)}
pub(super) fn validate_claim(db:&Connection,run:RunId,claim:&Claim,epoch:u64,time:u64)->Result<()> {
    if claim.epoch!=epoch{return Err(Error::Denied);}let graph=graph(db,run)?;graph.validate_claim(claim,time)?;
    let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1 AND state='active' AND owner_epoch=?2)",params![run.0.to_string(),sql_integer(epoch)?],|r|r.get(0))?;if !valid{return Err(Error::Denied);}Ok(())
}
pub(super) fn proposal(tx:&Transaction<'_>,run:RunId,p:&ToolProposal,epoch:u64)->Result<()>{
    if !exists(tx,run)?{return Ok(());}let graph=graph(tx,run)?;let node=graph.node(p.task)?;let claim=node.lease.as_ref().ok_or(Error::Denied)?;let time=now()?;
    if p.actor!=claim.actor||!p.preconditions.contains(&claim.precondition()){return Err(Error::Denied);}validate_claim(tx,run,claim,epoch,time)?;
    let resources=if p.descriptor_digest==crate::tools::files::DESCRIPTOR{
        use crate::tools::files::FileAction;let action:FileAction=serde_json::from_value(p.arguments.clone()).map_err(|_|Error::Invalid)?;
        match action{FileAction::Write{path,..}|FileAction::Patch{path,..}|FileAction::Delete{path,..}=>vec![Resource::Path(path)],FileAction::Rename{from,to,..}=>vec![Resource::Path(from),Resource::Path(to)],_=>vec![]}
    }else if p.descriptor_digest==crate::tools::git::DESCRIPTOR{let request:crate::tools::git::GitRequest=serde_json::from_value(p.arguments.clone()).map_err(|_|Error::Invalid)?;if request.writable(){vec![Resource::Git]}else{vec![]}}
    else if p.descriptor_digest==crate::mcp::DESCRIPTOR{
        // Third-party effects have no qualified workspace lease projection yet.
        return Err(Error::Unsupported);
    }else if p.descriptor_digest==crate::tools::process::DESCRIPTOR{vec![]}else{return Err(Error::Unsupported);};
    graph.mutation_allowed(claim,&resources,time)
}
fn task_uncertain(tx:&Transaction<'_>,task:TaskId)->Result<bool>{Ok(tx.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE task_id=?1 AND state NOT IN('terminal','denied','reconciled-observed','abandoned-unknown')) OR EXISTS(SELECT 1 FROM inference_assignments a JOIN inference_reservations i ON a.request_id=i.request_id WHERE a.task_id=?1 AND i.state IN('reserved','unknown') AND NOT EXISTS(SELECT 1 FROM recovery_resolutions x WHERE x.entity_id=i.request_id AND x.entity_kind='request' AND x.source_digest=i.request_digest))",[task.0.to_string()],|r|r.get(0))?)}
fn finish(tx:&Transaction<'_>,graph:&mut Graph,completion:Completion,epoch:u64,time:u64)->Result<()>{
    if completion.claim.epoch!=epoch||task_uncertain(tx,completion.claim.task)?{return Err(Error::Recovery);}graph.finish(&completion.claim,time,completion.success,completion.review)
}
fn record_evidence(tx:&Transaction<'_>,state:&Dir,graph:&mut Graph,claim:&Claim,operation:OperationId,epoch:u64,time:u64)->Result<()>{
    if claim.epoch!=epoch{return Err(Error::Denied);}graph.validate_claim(claim,time)?;
    let (task,actor,state_name,descriptor):(String,String,String,String)=tx.query_row("SELECT task_id,actor_id,state,descriptor_digest FROM operations WHERE id=?1",[operation.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    if task!=claim.task.0.to_string()||actor!=claim.actor.0.to_string()||state_name!="terminal"{return Err(Error::Denied);}
    let receipt:String=tx.query_row("SELECT payload FROM events WHERE entity_id=?1 AND kind='receipt-recorded' ORDER BY sequence DESC LIMIT 1",[operation.0.to_string()],|r|r.get(0))?;let receipt:ExecutionReceipt=serde_json::from_str(&receipt).map_err(|_|Error::State)?;
    let store=crate::artifacts::Store::open(state,false)?.ok_or(Error::State)?;
    let mut refs=tx.prepare("SELECT a.digest,a.size,a.media_type FROM artifacts a JOIN operation_artifacts o ON a.digest=o.digest WHERE o.operation_id=?1 AND a.media_type=?2")?;
    let rows=refs.query_map(params![operation.0.to_string(),OBSERVATION_MEDIA],|r|Ok(crate::artifacts::ArtifactRef{digest:r.get(0)?,size:r.get::<_,i64>(1)?.try_into().map_err(|_|rusqlite::Error::InvalidQuery)?,media_type:r.get(2)?}))?;
    let mut observed=vec![];for row in rows{let reference=row?;let observation:Observation=serde_json::from_slice(&store.read(&reference)?).map_err(|_|Error::State)?;if !receipt.artifacts.contains(&reference.digest){return Err(Error::Denied);}
        match &observation{
            Observation::File{revisions}=>{if descriptor!=crate::tools::files::DESCRIPTOR||receipt.outcome!=Outcome::Succeeded||revisions.len()!=1||receipt.after_revision.as_ref()!=revisions.first().map(|r|&r.sha256){return Err(Error::Denied);}},
            Observation::Check{projection,exit_code,..}=>{let intent:String=tx.query_row("SELECT metadata FROM operation_intents WHERE operation_id=?1",[operation.0.to_string()],|r|r.get(0))?;let intent:crate::recovery::Intent=serde_json::from_str(&intent).map_err(|_|Error::State)?;if descriptor!=crate::tools::process::DESCRIPTOR||receipt.exit_code!=*exit_code||receipt.after_revision.as_ref()!=Some(projection)||!matches!(intent,crate::recovery::Intent::Check{projection_sha256} if &projection_sha256==projection){return Err(Error::Denied);}}
        }observed.push((reference,observation));}
    for (_,observation) in &observed{if let Observation::File{revisions}=observation{for revision in revisions{for other in graph.nodes.iter_mut().filter(|n|n.task.id!=claim.task){if matches!(other.task.criterion,Criterion::Checked{..})&&other.evidence.revisions.iter().any(|r|r.path==revision.path&&r.sha256!=revision.sha256)&&!other.stale.contains(&revision.path){other.stale.push(revision.path.clone());}}}}}
    let node=graph.nodes.iter_mut().find(|n|n.task.id==claim.task).ok_or(Error::Invalid)?;
    for (reference,observation) in observed{match observation{
            Observation::File{revisions}=>for revision in revisions{node.stale.retain(|path|*path!=revision.path);node.evidence.revisions.retain(|r|r.path!=revision.path);node.evidence.revisions.push(revision);},
            Observation::Check{projection,revisions,exit_code:Some(0),truncated:false,cancelled:false} if receipt.outcome==Outcome::Succeeded=>{let evidence=CheckEvidence{operation,artifact:reference.digest,projection,revisions};if !node.evidence.checks.contains(&evidence){node.evidence.checks.push(evidence);}},_=>()
        }
    }if node.evidence.revisions.len()>32||node.evidence.checks.len()>32{return Err(Error::Unsupported);}Ok(())
}
pub(super) fn apply(tx:&Transaction<'_>,state:&Dir,command:Command,revision:u64,epoch:u64)->Result<()> {
    let run=match &command{Command::NextSequence{run,..}|Command::Inbox{run,..}|Command::Withhold{run,..}|Command::Admit{run,..}|Command::Inspect{run,..}|Command::Claim{run,..}|Command::Yield{run,..}|Command::Finish{run,..}|Command::RecordEvidence{run,..}|Command::Release{run,..}|Command::Expire{run,..}|Command::Reconcile{run,..}|Command::Forward{run,..}|Command::Consent{run,..}|Command::Send{run,..}|Command::Ack{run,..}=>*run};
    if let Command::NextSequence{sender,recipient,result,..}=command{let value:i64=tx.query_row("SELECT coalesce(max(sequence),0)+1 FROM messages WHERE run_id=?1 AND sender=?2 AND recipient=?3",params![run.0.to_string(),sender.0.to_string(),recipient.0.to_string()],|r|r.get(0))?;*result.lock().map_err(|_|Error::State)?=Some(value.try_into().map_err(|_|Error::State)?);return Ok(());}
    if let Command::Inbox{recipient,result,..}=command{
        let graph=graph(tx,run)?;if !actor(&graph,recipient){return Err(Error::Denied);}let mut values=vec![];let mut q=tx.prepare("SELECT envelope FROM messages WHERE run_id=?1 AND recipient=?2 AND ack_digest IS NULL ORDER BY rowid LIMIT 257")?;
        for row in q.query_map(params![run.0.to_string(),recipient.0.to_string()],|r|r.get::<_,String>(0))?{if values.len()>=256{return Err(Error::Unsupported);}let message:MessageEnvelope<Payload>=serde_json::from_str(&row?).map_err(|_|Error::State)?;let permitted=delivery_allowed(tx,run,&message,now()?)?;values.push(crate::coordination::Delivery{message,permitted});}*result.lock().map_err(|_|Error::State)?=values;return Ok(());
    }
    if let Command::Inspect{result,..}=command{*result.lock().map_err(|_|Error::State)?=Some(graph(tx,run)?);return Ok(());}
    if let Command::Admit{plan,..}=command{
        if revision!=0{return Err(Error::Conflict);}let graph=Graph::new(plan)?;let root=graph.plan.root();let owner:String=tx.query_row("SELECT owner_agent_id FROM tasks WHERE id=?1 AND run_id=?2",params![root.id.0.to_string(),run.0.to_string()],|r|r.get(0))?;if owner!=root.actor.0.to_string(){return Err(Error::Denied);}
        for agent in &graph.plan.agents{let role=serde_json::to_value(&agent.role).map_err(|_|Error::State)?;if agent.id==root.actor{tx.execute("UPDATE agents SET role=?1 WHERE id=?2 AND run_id=?3",params![role.as_str(),agent.id.0.to_string(),run.0.to_string()])?;}else{tx.execute("INSERT INTO agents VALUES(?1,?2,?3)",params![agent.id.0.to_string(),run.0.to_string(),role.as_str()])?;}}
        for node in &graph.nodes{if node.task.id!=root.id{tx.execute("INSERT INTO tasks VALUES(?1,?2,?3,'pending',0)",params![node.task.id.0.to_string(),run.0.to_string(),node.task.actor.0.to_string()])?;}}
        tx.execute("UPDATE runs SET mode=?1 WHERE id=?2",params![serde_json::to_value(&graph.plan.mode).map_err(|_|Error::State)?.as_str(),run.0.to_string()])?;
        let document=serde_json::to_string(&graph).map_err(|_|Error::State)?;ensure_space(state,document.len() as u64*3+65536)?;
        tx.execute("INSERT INTO coordination VALUES(?1,?2,0,?3)",params![run.0.to_string(),root.id.0.to_string(),document])?;audit(tx,&run.0.to_string(),"supervisor","hierarchy-admitted")?;return Ok(());
    }
    if let Command::Forward{message,..}=&command{if let Ok(envelope)=tx.query_row("SELECT envelope FROM messages WHERE id=?1 AND run_id=?2",params![message.id.0.to_string(),run.0.to_string()],|r|r.get::<_,String>(0)){let stored:MessageEnvelope<Payload>=serde_json::from_str(&envelope).map_err(|_|Error::State)?;let mut retry=message.clone();retry.sequence=stored.sequence;if serde_json::to_string(&retry).map_err(|_|Error::State)?==envelope{return Ok(());}return Err(Error::Conflict);}}
    // Stable duplicate transport acknowledgments are no-ops, including their original transition.
    if let Command::Send{message,..}=&command{if let Ok(digest)=tx.query_row("SELECT digest FROM messages WHERE id=?1 AND run_id=?2",params![message.id.0.to_string(),run.0.to_string()],|r|r.get::<_,String>(0)){if digest==crate::tools::sha256(&serde_json::to_vec(message).map_err(|_|Error::Invalid)?){return Ok(());}return Err(Error::Conflict);}}
    if let Command::Ack{id,recipient,context,completion,..}=&command{if let Ok((Some(digest),stored))=tx.query_row("SELECT ack_digest,recipient FROM messages WHERE id=?1 AND run_id=?2",params![id.0.to_string(),run.0.to_string()],|r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,String>(1)?))){if stored==recipient.0.to_string()&&digest==crate::tools::sha256(&serde_json::to_vec(&(context,completion)).map_err(|_|Error::Invalid)?){return Ok(());}return Err(Error::Conflict);}}
    let mut graph=graph(tx,run)?;if graph.revision!=revision{return Err(Error::Conflict);}
    let kind=match command{
        Command::Claim{task,now,expires,..}=>{let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM runs WHERE id=?1 AND state='active' AND owner_epoch=?2)",params![run.0.to_string(),sql_integer(epoch)?],|r|r.get(0))?;if !active{return Err(Error::Denied);}let deadline=crate::lifecycle::snapshot(tx,run)?.spec.deadline;if expires>deadline{return Err(Error::Denied);}let sequence:i64=tx.query_row("SELECT next_event_sequence FROM meta",[],|r|r.get(0))?;graph.claim(task,epoch,now,expires,sequence.try_into().map_err(|_|Error::State)?)?;"task-claimed"},
        Command::Yield{claim,now,..}=>{if claim.epoch!=epoch||task_uncertain(tx,claim.task)?{return Err(Error::Recovery);}graph.yield_step(&claim,now)?;"task-yielded"},
        Command::Finish{completion,now,..}=>{finish(tx,&mut graph,completion,epoch,now)?;"task-finished"},
        Command::RecordEvidence{claim,operation,now,..}=>{record_evidence(tx,state,&mut graph,&claim,operation,epoch,now)?;"task-evidence"},
        Command::Release{claim,..}=>{if claim.epoch!=epoch||task_uncertain(tx,claim.task)?{return Err(Error::Recovery);}graph.release(&claim)?;"task-released-after-drain"},
        Command::Expire{now,..}=>{graph.expire(now);"task-lease-expired"},
        Command::Reconcile{task,..}=>{let node=graph.node(task)?;let lease=node.lease.as_ref().ok_or(Error::Conflict)?;let proof:Option<i64>=tx.query_row("SELECT max(sequence) FROM events WHERE kind IN('owner-fenced-recovered','restore-committed-no-replay')",[],|r|r.get(0))?;if task_uncertain(tx,task)?||proof.is_none_or(|p|p<=lease.issued_sequence as i64){return Err(Error::Recovery);}graph.reconcile(task)?;"task-reconciled"},
        Command::Forward{consent,message,now:time,..}=>{
            if consent.reference!=match &message.payload{Payload::Decision{reference,..}=>reference.clone(),_=>return Err(Error::Denied)}||consent.recipient!=message.recipient||consent.epoch!=permission_epoch(tx,run)?||consent.expiry<=time{return Err(Error::Denied);}
            tx.execute("INSERT INTO forwarding_consents VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT DO UPDATE SET expiry=max(expiry,excluded.expiry)",params![run.0.to_string(),consent.reference,consent.recipient.0.to_string(),consent.scope,sql_integer(consent.epoch)?,sql_integer(consent.expiry)?])?;send(tx,&graph,run,&message,epoch,time)?;"user-decision-forwarded"
        },
        Command::Consent{consent,..}=>{if !actor(&graph,consent.recipient)||consent.epoch!=permission_epoch(tx,run)?||consent.expiry<=now()?||consent.reference.len()!=64||consent.scope.is_empty()||consent.scope.len()>256{return Err(Error::Denied);}tx.execute("INSERT INTO forwarding_consents VALUES(?1,?2,?3,?4,?5,?6)",params![run.0.to_string(),consent.reference,consent.recipient.0.to_string(),consent.scope,sql_integer(consent.epoch)?,sql_integer(consent.expiry)?])?;"forwarding-consented"},
        Command::Send{message,now:time,..}=>{send(tx,&graph,run,&message,epoch,time)?;"message-enqueued"},
        Command::Withhold{id,recipient,..}=>{
            let envelope:String=tx.query_row("SELECT envelope FROM messages WHERE id=?1 AND run_id=?2 AND recipient=?3 AND ack_digest IS NULL",params![id.0.to_string(),run.0.to_string(),recipient.0.to_string()],|r|r.get(0))?;let message:MessageEnvelope<Payload>=serde_json::from_str(&envelope).map_err(|_|Error::State)?;if delivery_allowed(tx,run,&message,now()?)?{return Err(Error::Denied);}tx.execute("UPDATE messages SET ack_digest='withheld-after-consent-expiry-or-revocation' WHERE id=?1",[id.0.to_string()])?;"message-withheld"
        },
        Command::Ack{id,recipient,context,completion,now,..}=>{
            if !actor(&graph,recipient)||context.actor!=recipient||context.epoch!=epoch{return Err(Error::Denied);}graph.validate_claim(&context,now)?;let actual:String=tx.query_row("SELECT recipient FROM messages WHERE id=?1 AND run_id=?2 AND ack_digest IS NULL",params![id.0.to_string(),run.0.to_string()],|r|r.get(0))?;if actual!=recipient.0.to_string(){return Err(Error::Denied);}
            let envelope:String=tx.query_row("SELECT envelope FROM messages WHERE id=?1",[id.0.to_string()],|r|r.get(0))?;let message:MessageEnvelope<Payload>=serde_json::from_str(&envelope).map_err(|_|Error::State)?;if !delivery_allowed(tx,run,&message,now)?{return Err(Error::Denied);}
            let digest=crate::tools::sha256(&serde_json::to_vec(&(&context,&completion)).map_err(|_|Error::Invalid)?);if let Some(completion)=completion{if completion.claim!=context||completion.claim.actor!=recipient{return Err(Error::Denied);}finish(tx,&mut graph,completion,epoch,now)?;}
            let node=graph.nodes.iter_mut().find(|n|n.task.id==context.task).ok_or(Error::Invalid)?;if !node.received.contains(&id){if node.received.len()>=128{return Err(Error::Unsupported);}node.received.push(id);}
            tx.execute("UPDATE messages SET ack_digest=?1 WHERE id=?2",params![digest,id.0.to_string()])?;"message-acknowledged"
        },Command::Admit{..}|Command::Inspect{..}|Command::Inbox{..}|Command::NextSequence{..}=>unreachable!()
    };
    graph.revision=graph.revision.checked_add(1).ok_or(Error::State)?;let document=serde_json::to_string(&graph).map_err(|_|Error::State)?;if document.len()>512*1024{return Err(Error::Unsupported);}ensure_space(state,document.len() as u64*3+65536)?;
    let changed=tx.execute("UPDATE coordination SET document=?1,revision=?2 WHERE run_id=?3 AND revision=?4",params![document,sql_integer(graph.revision)?,run.0.to_string(),sql_integer(revision)?])?;if changed!=1{return Err(Error::Conflict);}
    for node in &graph.nodes{let state=serde_json::to_value(node.state).map_err(|_|Error::State)?;tx.execute("UPDATE tasks SET state=?1,revision=?2,owner_agent_id=?3 WHERE id=?4 AND run_id=?5",params![state.as_str(),sql_integer(node.revision)?,node.task.actor.0.to_string(),node.task.id.0.to_string(),run.0.to_string()])?;}
    audit(tx,&run.0.to_string(),"supervisor",kind)
}
fn send(tx:&Transaction<'_>,graph:&Graph,run:RunId,message:&MessageEnvelope<Payload>,_epoch:u64,time:u64)->Result<()>{
    if message.schema_version!=1||message.sequence==0||!actor(graph,message.sender)||!actor(graph,message.recipient)||(!graph.plan.recipients.contains(&(message.sender,message.recipient))&&!matches!(message.payload,Payload::Decision{..}))||graph.node(message.task).is_err()||message.evidence.len()>32||message.user_decisions.len()>1{return Err(Error::Denied);}
    if let Some(causal)=message.causal_id{let found:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1 AND run_id=?2)",params![causal.0.to_string(),run.0.to_string()],|r|r.get(0))?;if !found{return Err(Error::Conflict);}}
    let(total,pending):(i64,i64)=tx.query_row("SELECT count(*),coalesce(sum(ack_digest IS NULL),0) FROM messages WHERE run_id=?1",[run.0.to_string()],|r|Ok((r.get(0)?,r.get(1)?)))?;if total>=1024||pending>=256{return Err(Error::Denied);}
    let previous:i64=tx.query_row("SELECT coalesce(max(sequence),0) FROM messages WHERE run_id=?1 AND sender=?2 AND recipient=?3",params![run.0.to_string(),message.sender.0.to_string(),message.recipient.0.to_string()],|r|r.get(0))?;if sql_integer(message.sequence)?!=previous+1{return Err(Error::Conflict);}
    match &message.payload{
        Payload::Decision{reference,scope,text}=>{if text.len()>4096||reference!=&crate::tools::sha256(text.as_bytes())||scope.is_empty()||scope.len()>256||message.user_decisions!=[reference.clone()]{return Err(Error::Denied);}let allowed:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM forwarding_consents WHERE run_id=?1 AND reference=?2 AND recipient=?3 AND scope=?4 AND epoch=?5 AND expiry>?6)",params![run.0.to_string(),reference,message.recipient.0.to_string(),scope,sql_integer(permission_epoch(tx,run)?)?,sql_integer(time)?],|r|r.get(0))?;if !allowed{return Err(Error::Denied);}},
        Payload::Progress{summary}=>if !message.user_decisions.is_empty()||summary.len()>4096{return Err(Error::Denied);},
        Payload::Evidence{references}=>if !message.user_decisions.is_empty()||references.len()>32{return Err(Error::Denied);}
    }
    let envelope=serde_json::to_string(message).map_err(|_|Error::Invalid)?;if envelope.len()>8192{return Err(Error::Unsupported);}tx.execute("INSERT INTO messages VALUES(?1,?2,?3,?4,?5,?6,?7,NULL)",params![message.id.0.to_string(),run.0.to_string(),message.sender.0.to_string(),message.recipient.0.to_string(),sql_integer(message.sequence)?,crate::tools::sha256(envelope.as_bytes()),envelope])?;Ok(())
}

fn permission_epoch(tx:&Transaction<'_>,run:RunId)->Result<u64>{Ok(crate::lifecycle::snapshot(tx,run)?.spec.permission_epoch)}
fn delivery_allowed(tx:&Transaction<'_>,run:RunId,message:&MessageEnvelope<Payload>,time:u64)->Result<bool>{
    if let Payload::Decision{reference,scope,..}=&message.payload{Ok(tx.query_row("SELECT EXISTS(SELECT 1 FROM forwarding_consents WHERE run_id=?1 AND reference=?2 AND recipient=?3 AND scope=?4 AND epoch=?5 AND expiry>?6)",params![run.0.to_string(),reference,message.recipient.0.to_string(),scope,sql_integer(permission_epoch(tx,run)?)?,sql_integer(time)?],|r|r.get(0))?)}else{Ok(true)}
}
