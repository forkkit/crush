use crate::lang::{Argument, Value, List, ValueType, Dict};
use crate::errors::{CrushResult, argument_error};
use std::path::Path;

pub fn two_arguments(arguments: &Vec<Argument>) -> CrushResult<()> {
    if arguments.len() != 2 {
        argument_error("Expected exactly two argument")
    } else {
        Ok(())
    }
}

pub fn single_argument(arguments: &Vec<Argument>) -> CrushResult<()> {
    if arguments.len() != 1 {
        argument_error("Expected exactly one argument")
    } else {
        Ok(())
    }
}

pub fn argument_files(mut arguments: Vec<Argument>) -> CrushResult<Vec<Box<Path>>> {
    let mut files = Vec::new();
    for a in arguments.drain(..) {
        a.value.file_expand(&mut files);
    }
    Ok(files)
}

pub fn single_argument_type(mut arg: Vec<Argument>) -> CrushResult<ValueType> {
    match arg.len() {
        1 => {
            let a = arg.remove(0);
            match (a.name, a.value) {
                (None, Value::Type(t)) => Ok(t),
                _ => argument_error("Expected a list value"),
            }
        }
        _ => argument_error("Expected a single value"),
    }
}

pub fn single_argument_list(mut arg: Vec<Argument>) -> CrushResult<List> {
    match arg.len() {
        1 => {
            let a = arg.remove(0);
            match (a.name, a.value) {
                (None, Value::List(t)) => Ok(t),
                _ => argument_error("Expected a list value"),
            }
        }
        _ => argument_error("Expected a single value"),
    }
}

pub fn single_argument_dict(mut arg: Vec<Argument>) -> CrushResult<Dict> {
    match arg.len() {
        1 => {
            let a = arg.remove(0);
            match (a.name, a.value) {
                (None, Value::Dict(t)) => Ok(t),
                _ => argument_error("Expected a list value"),
            }
        }
        _ => argument_error("Expected a single value"),
    }
}

pub fn single_argument_field(mut arg: Vec<Argument>) -> CrushResult<Vec<Box<str>>> {
    match arg.len() {
        1 => {
            let a = arg.remove(0);
            match (a.name, a.value) {
                (None, Value::Field(t)) => Ok(t),
                _ => argument_error("Expected a field value"),
            }
        }
        _ => argument_error("Expected a single value"),
    }
}

pub fn single_argument_text(mut arg: Vec<Argument>) -> CrushResult<Box<str>> {
    match arg.len() {
        1 => {
            let a = arg.remove(0);
            match (a.name, a.value) {
                (None, Value::Text(t)) => Ok(t),
                _ => argument_error("Expected a text value"),
            }
        }
        _ => argument_error("Expected a single value"),
    }
}

pub fn single_argument_integer(mut arg: Vec<Argument>) -> CrushResult<i128> {
    match arg.len() {
        1 => {
            let a = arg.remove(0);
            match (a.name, a.value) {
                (None, Value::Integer(i)) => Ok(i),
                _ => argument_error("Expected a text value"),
            }
        }
        _ => argument_error("Expected a single value"),
    }
}

pub fn optional_argument_integer(mut arg: Vec<Argument>) -> CrushResult<Option<i128>> {
    match arg.len() {
        0 => Ok(None),
        1 => {
            let a = arg.remove(0);
            match (a.name, a.value) {
                (None, Value::Integer(i)) => Ok(Some(i)),
                _ => argument_error("Expected a text value"),
            }
        }
        _ => argument_error("Expected a single value"),
    }
}