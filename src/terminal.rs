//! Line-oriented trusted UI. Input thread never blocks supervisor cancellation.
use crate::{chat::{Review,Reviewer},error::{Error,Result}};
use std::{future::Future,pin::Pin,io::{BufRead,Read,Write}};
pub struct Terminal {input:tokio::sync::mpsc::Receiver<String>}
impl Terminal {
    pub fn new()->Self {
        let (tx,input)=tokio::sync::mpsc::channel(1);
        std::thread::spawn(move||{
            let stdin=std::io::stdin();let mut input=stdin.lock();
            loop {
                let mut line=String::new();
                // At most 4 KiB is ever buffered; overlong input cannot approve.
                match input.by_ref().take(4097).read_line(&mut line){
                    Ok(0)|Err(_)=>break,
                    Ok(_) if line.len()>4096=>break,
                    Ok(_)=>if tx.blocking_send(line).is_err(){break;},
                }
            }
        });Self{input}
    }
}
impl Default for Terminal{fn default()->Self{Self::new()}}
impl Reviewer for Terminal {
    fn approve(&mut self,review:Review)->Pin<Box<dyn Future<Output=Result<bool>>+Send+'_>> {
        Box::pin(async move {
            // JSON escapes all terminal control characters from untrusted source/model content.
            let encoded=serde_json::to_string(&review).map_err(|_|Error::Invalid)?;
            let challenge=crate::tools::sha256(encoded.as_bytes());
            println!("{encoded}");
            print!("Type approve {challenge} to authorize this exact request, or anything else to stop: ");std::io::stdout().flush()?;
            Ok(self.input.recv().await.is_some_and(|s|s.trim()==format!("approve {challenge}")))
        })
    }
}
