//! Read-only llmfit 1.1.17 CLI JSON import. Never executes verify_command, tools, downloads or benchmarks.
use crate::{error::{Error,Result},model_pool::{Policy,State,Behavior}};
use serde::{Serialize,Deserialize};
#[derive(Clone,Debug,Deserialize)]
pub struct Fit {pub name:String,pub usable_context:u32,pub effective_context_length:u32,pub score:f64,pub estimated_tps:f64,pub installed:bool,pub runtime:String,pub capability_ids:Vec<String>,pub total_memory_gb:f64}
#[derive(Deserialize)]
struct Input {models:Vec<Fit>,system:serde_json::Value}
#[derive(Clone,Debug,Serialize)]
pub struct Catalog {pub source_version:String,pub source_digest:String,#[serde(skip)]pub models:Vec<Fit>}
pub fn import(version:&str,bytes:&[u8])->Result<Catalog>{
    if version!="1.1.17"{return Err(Error::Unsupported);}if bytes.len()>65536{return Err(Error::Unsupported);}let input:Input=serde_json::from_slice(bytes).map_err(|_|Error::Invalid)?;
    if !input.system.is_object()||input.models.is_empty()||input.models.len()>128{return Err(Error::Invalid);}let mut names=std::collections::HashSet::new();for fit in &input.models{if fit.name.is_empty()||fit.name.len()>256||!names.insert(&fit.name)||fit.usable_context==0||fit.effective_context_length==0||!fit.score.is_finite()||!(0.0..=100.0).contains(&fit.score)||!fit.estimated_tps.is_finite()||fit.estimated_tps<0.0||!fit.total_memory_gb.is_finite()||fit.total_memory_gb<=0.0||fit.total_memory_gb>1e6{return Err(Error::Invalid);}}
    Ok(Catalog{source_version:version.into(),source_digest:crate::tools::sha256(bytes),models:input.models})
}
#[derive(Clone,Copy,Debug,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Preference {Efficient,Balanced,Ambitious}
#[derive(Clone,Debug,Serialize)]
pub struct Recommendation {pub allocation_digest:String,pub feasible:bool,pub rank_score:Option<f64>,pub peak_ram:u64,pub peak_vram:u64,pub reasons:Vec<String>,pub source_digest:String}
pub fn rank(catalog:&Catalog,allocations:&[Policy],actors:usize,tasks:usize,preference:Preference)->Result<Vec<Recommendation>>{
    if allocations.is_empty()||allocations.len()>32{return Err(Error::Invalid);}let mut output=vec![];
    for policy in allocations{
        let allocation_digest=crate::model_pool::digest(policy)?;let mut reasons=vec![];let mut peak_ram=0;let mut peak_vram=0;let mut scores=vec![];
        let evaluated=(||{
            let state=State::new(policy.clone(),1,actors,tasks)?;let(base_ram,base_vram)=state.usage()?;peak_ram=base_ram;peak_vram=base_vram;
            for id in &policy.assignments{let profile=policy.profile(id)?;let fit=catalog.models.iter().find(|f|f.name==profile.provider.model).ok_or(Error::Unsupported)?;if !fit.installed||fit.runtime!="llama.cpp"||!fit.capability_ids.iter().any(|c|c=="tool_use")||profile.provider.context_tokens.is_none_or(|ctx|ctx>fit.usable_context){return Err(Error::Unsupported);}scores.push(fit.score);let context=profile.provider.context_tokens.ok_or(Error::Unsupported)? as u64;let declared=profile.weights_ram.checked_add(profile.weights_vram).and_then(|n|n.checked_add(context.checked_mul(profile.kv_ram_per_token+profile.kv_vram_per_token)?)).ok_or(Error::Invalid)?;if (declared as f64)<fit.total_memory_gb*1e9{return Err(Error::Denied);}}
            if policy.behavior==Behavior::Relay{
                for phase in &policy.phases{let p=policy.profile(&phase.profile)?;let ctx=p.provider.context_tokens.ok_or(Error::Unsupported)? as u64;let extra_ram=ctx.checked_mul(p.kv_ram_per_token).and_then(|n|n.checked_add(if p.resident{0}else{p.weights_ram+p.loading_ram+policy.limits.loading_overlap_ram})).ok_or(Error::Invalid)?;let extra_vram=ctx.checked_mul(p.kv_vram_per_token).and_then(|n|n.checked_add(if p.resident{0}else{p.weights_vram+p.loading_vram+policy.limits.loading_overlap_vram})).ok_or(Error::Invalid)?;peak_ram=peak_ram.max(base_ram.checked_add(extra_ram).ok_or(Error::Invalid)?);peak_vram=peak_vram.max(base_vram.checked_add(extra_vram).ok_or(Error::Invalid)?);}
                reasons.push("Serial relay phases; evidence handoffs only. Unloaded targets require explicit activation and observed completion.".into());
            }else{
                let mut instances=std::collections::HashSet::new();for id in &policy.assignments{let p=policy.profile(id)?;if instances.insert(&p.instance){let actors=policy.assignments.iter().filter(|id|policy.profile(id).is_ok_and(|v|v.instance==p.instance)).count() as u64;let slots=actors.min(p.slots as u64);let ctx=p.provider.context_tokens.ok_or(Error::Unsupported)? as u64;peak_ram=peak_ram.checked_add(ctx.checked_mul(p.kv_ram_per_token).and_then(|n|n.checked_mul(slots)).ok_or(Error::Invalid)?).ok_or(Error::Invalid)?;peak_vram=peak_vram.checked_add(ctx.checked_mul(p.kv_vram_per_token).and_then(|n|n.checked_mul(slots)).ok_or(Error::Invalid)?).ok_or(Error::Invalid)?;}}
                reasons.push("Distinct role contexts; shared weights counted once per declared actual instance, KV counted per active request; queued roles share capacity.".into());
            }
            if peak_ram>policy.limits.ram||peak_vram>policy.limits.vram{return Err(Error::Denied);}Ok(())
        })();
        let feasible=evaluated.is_ok();let score=if feasible{let minimum=scores.iter().copied().reduce(f64::min).ok_or(Error::Invalid)?;let average=scores.iter().sum::<f64>()/scores.len() as f64;let efficient=100.0*(1.0-peak_ram as f64/policy.limits.ram as f64).max(0.0);Some(match preference{Preference::Efficient=>0.8*efficient+0.2*minimum,Preference::Balanced=>0.4*efficient+0.6*minimum,Preference::Ambitious=>average})}else{reasons.push(format!("Unsupported allocation: {}; no acquisition or fallback.",evaluated.err().ok_or(Error::State)?.kind()));None};
        reasons.push("Scores and throughput are imported llmfit estimates, not Rivet model measurements. Owner capacity/memory declarations remain separate inputs.".into());output.push(Recommendation{allocation_digest,feasible,rank_score:score,peak_ram,peak_vram,reasons,source_digest:catalog.source_digest.clone()});
    }
    output.sort_by(|a,b|b.feasible.cmp(&a.feasible).then_with(||b.rank_score.unwrap_or(-1.0).total_cmp(&a.rank_score.unwrap_or(-1.0))).then_with(||a.allocation_digest.cmp(&b.allocation_digest)));Ok(output)
}
