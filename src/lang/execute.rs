use crate::lang::errors::{CrushResult, to_crush_error, argument_error, CrushError};
use crate::lang::printer::Printer;
use crate::lang::scope::Scope;
use std::{fs, thread};
use crate::lang::parser::parse;
use crate::lang::execution_context::{JobContext, ExecutionContext};
use crate::lang::stream::{empty_channel, ValueSender, black_hole, channels};
use std::path::Path;
use crate::lang::serialization::{deserialize, serialize_file, serialize};
use crate::lang::value::Value;
use std::io::Write;
use std::any::Any;

pub fn file(global_env: Scope, filename: &Path, printer: &Printer, output: &ValueSender) -> CrushResult<()> {
    let cmd = to_crush_error(fs::read_to_string(filename))?;
    string(global_env, &cmd.as_str(), printer, output);
    Ok(())
}

pub fn pup(env: Scope, buf: &Vec<u8>, printer: &Printer, output: &ValueSender) -> CrushResult<()> {
    let cmd = deserialize(buf, &env)?;
    match cmd {
        Value::Command(cmd) => {
            let (snd, recv) = channels();

            let t: std::thread::JoinHandle<std::result::Result<(), CrushError>> =
                to_crush_error(
                    thread::Builder::new().name("serializer".to_string()).spawn(move || {
                        let val = recv.recv()?;
                        let mut buf = Vec::new();
                        serialize(&val.materialize(), &mut buf)?;
                        to_crush_error(std::io::stdout().write(&buf))?;
                        Ok(())
                    }))?;

            cmd.invoke(
                ExecutionContext {
                    input: empty_channel(),
                    output: snd,
                    arguments: vec![],
                    env,
                    this: None,
                    printer: printer.clone(),
                }
            )?;

            match t.join() {
                Ok(_) => Ok(()),
                Err(e) => argument_error("Error while waiting for output"),
            }
        }
        _ => argument_error("Expected a command, but found other value"),
    }
}

pub fn string(global_env: Scope, s: &str, printer: &Printer, output: &ValueSender) {
    match parse(s, &global_env) {
        Ok(jobs) => {
            for job_definition in jobs {
                match job_definition.invoke(JobContext::new(
                    empty_channel(), output.clone(), global_env.clone(), printer.clone())) {
                    Ok(handle) => {
                        handle.join(&printer);
                    }
                    Err(e) => printer.crush_error(e),
                }
            }
        }
        Err(error) => {
            printer.crush_error(error);
        }
    }
}
