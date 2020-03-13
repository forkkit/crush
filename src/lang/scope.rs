use crate::lang::errors::{error, CrushResult};
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::lang::{value::Value, value::ValueType};
use std::collections::HashMap;

/**
  This is where we store variables, including functions.

  The data is protected by a mutex, in order to make sure that all threads can read and write
  concurrently.

  The data is protected by an Arc, in order to make sure that it gets deallocated and can be shared
  across threads.

  In order to ensure that there are no deadlocks, a given thread will only ever lock one scope at a
  time. This forces us to manually drop some variables.
*/
#[derive(Clone)]
#[derive(Debug)]
pub struct Scope {
    data: Arc<Mutex<ScopeData>>,
}

#[derive(Debug)]
struct ScopeData {
    /** This is the parent scope used to perform variable name resolution. If a variable lookup
     fails in the current scope, it proceeds to this scope. This is usually the scope in which this
     scope was *created*.
     */
    pub parent_scope: Option<Scope>,
    /** This is the scope in which the current scope was called. Since a closure can be called
     from inside any scope, it need not be the same as the parent scope. This scope is the one used
     for break/continue loop control. */
    pub calling_scope: Option<Scope>,

    /** This is a list of scopes that are imported into the current scope. Anything directly inside
    one of these scopes is also considered part of this scope. */
    pub uses: Vec<Scope>,

    /** The actual data of this scope. */
    pub mapping: HashMap<String, Value>,

    /** True if this scope is a loop. Required to implement the break/continue commands.*/
    pub is_loop: bool,

    /** True if this scope should stop execution, i.e. if the continue or break commands have been
    called.  */
    pub is_stopped: bool,

    /** True if this scope can not be further modified. Note that mutable variables in it, e.g.
    lists can still be modified. */
    pub is_readonly: bool,
}

impl ScopeData {
    fn new(parent_scope: Option<Scope>, calling_scope: Option<Scope>, is_loop: bool) -> ScopeData {
        return ScopeData {
            parent_scope,
            calling_scope,
            is_loop,
            uses: Vec::new(),
            mapping: HashMap::new(),
            is_stopped: false,
            is_readonly: false,
        };
    }
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            data: Arc::from(Mutex::new(ScopeData::new(None, None, false))),
        }
    }

    pub fn create_child(&self, caller: &Scope, is_loop: bool) -> Scope {
        Scope {
            data: Arc::from(Mutex::new(ScopeData::new(
                Some(self.clone()),
                Some(caller.clone()),
                is_loop))),
        }
    }

    pub fn do_continue(&self) -> bool {
        let data = self.data.lock().unwrap();
        if data.is_readonly {
            false
        } else if data.is_loop {
            true
        } else {
            let caller = data.calling_scope.clone();
            drop(data);
            let ok = caller
                .map(|p| p.do_continue())
                .unwrap_or(false);
            if !ok {
                false
            } else {
                self.data.lock().unwrap().is_stopped = true;
                true
            }
        }
    }

    pub fn do_break(&self) -> bool {
        let mut data = self.data.lock().unwrap();
        if data.is_readonly {
            false
        } else if data.is_loop {
            data.is_stopped = true;
            true
        } else {
            let caller = data.calling_scope.clone();
            drop(data);
            let ok = caller
                .map(|p| p.do_break())
                .unwrap_or(false);
            if !ok {
                false
            } else {
                self.data.lock().unwrap().is_stopped = true;
                true
            }
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.data.lock().unwrap().is_stopped
    }

    pub fn create_namespace(&self, name: &str) -> CrushResult<Scope> {
        let res = Scope {
            data: Arc::from(Mutex::new(ScopeData::new(None, None, false))),
        };
        self.declare(&[Box::from(name)], Value::Scope(res.clone()))?;
        Ok(res)
    }

    pub fn declare_str(&self, name: &str, value: Value) -> CrushResult<()> {
        let n = &name.split('.').map(|e: &str| Box::from(e)).collect::<Vec<Box<str>>>()[..];
        return self.declare(n, value);
    }

    pub fn declare(&self, name: &[Box<str>], value: Value) -> CrushResult<()> {
        if name.is_empty() {
            return error("Empty variable name");
        }
        if name.len() == 1 {
            let mut data = self.data.lock().unwrap();
            if data.is_readonly {
                return error("Scope is read only");
            }
            if data.mapping.contains_key(name[0].as_ref()) {
                return error(format!("Variable ${{{}}} already exists", name[0]).as_str());
            }
            data.mapping.insert(name[0].to_string(), value);
            Ok(())
        } else {
            match self.get(name[0].as_ref()) {
                None => error("Not a namespace"),
                Some(Value::Scope(env)) => env.declare(&name[1..name.len()], value),
                _ => error("Unknown namespace"),
            }
        }
    }

    pub fn set_str(&self, name: &str, value: Value) -> CrushResult<()> {
        let n = &name.split('.').map(|e: &str| Box::from(e)).collect::<Vec<Box<str>>>()[..];
        return self.set(n, value);
    }

    pub fn set(&self, name: &[Box<str>], value: Value) -> CrushResult<()> {
        if name.is_empty() {
            return error("Empty variable name");
        }
        if name.len() == 1 {
            self.set_on_data(name[0].as_ref(), value)
        } else {
            match self.get(name[0].as_ref()) {
                None => error("Not a namespace"),
                Some(Value::Scope(env)) => env.set(&name[1..name.len()], value),
                _ => error("Unknown namespace"),
            }
        }
    }

    fn set_on_data(&self, name: &str, value: Value) -> CrushResult<()> {
        let mut data = self.data.lock().unwrap();
        if !data.mapping.contains_key(name) {
            match data.parent_scope.clone() {
                Some(p) => {
                    drop(data);
                    p.set_on_data(name, value)
                }
                None => error(format!("Unknown variable ${{{}}}", name).as_str()),
            }
        } else {
            if data.is_readonly {
                error("Scope is read only")
            } else if data.mapping[name].value_type() != value.value_type() {
                error(format!("Type mismatch when reassigning variable ${{{}}}. Use `unset ${{{}}}` to remove old variable.", name, name).as_str())
            } else {
                data.mapping.insert(name.to_string(), value);
                Ok(())
            }
        }
    }

    pub fn remove_str(&self, name: &str) -> Option<Value> {
        let n = &name.split('.').map(|e: &str| Box::from(e)).collect::<Vec<Box<str>>>()[..];
        return self.remove(n);
    }

    pub fn remove(&self, name: &[Box<str>]) -> Option<Value> {
        if name.is_empty() {
            return None;
        }
        if name.len() == 1 {
            self.remove_here(name[0].as_ref())
        } else {
            match self.get(name[0].as_ref()) {
                None => None,
                Some(Value::Scope(env)) => env.remove(&name[1..name.len()]),
                _ => None,
            }
        }
    }

    fn remove_here(&self, key: &str) -> Option<Value> {
        let mut data = self.data.lock().unwrap();
        if !data.mapping.contains_key(key) {
            match data.parent_scope.clone() {
                Some(p) => {
                    drop(data);
                    p.remove_here(key)
                }
                None => None,
            }
        } else {
            if data.is_readonly {
                return None;
            }
            data.mapping.remove(key)
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let data = self.data.lock().unwrap();
        match data.mapping.get(&name.to_string()) {
            Some(v) => Some(v.clone()),
            None => match data.parent_scope.clone() {
                Some(p) => {
                    drop(data);
                    p.get(name)
                },
                None => {
                    let uses = data.uses.clone();
                    drop(data);
                    for used in &uses {
                        if let Some(res) = used.get(name) {
                            return Some(res);
                        }
                    }
                    None
                }
            }
        }
    }

    pub fn r#use(&self, other: &Scope) {
        self.data.lock().unwrap().uses.push(other.clone());
    }

    pub fn dump(&self, map: &mut HashMap<String, ValueType>) {
        match self.data.lock().unwrap().parent_scope.clone() {
            Some(p) => p.dump(map),
            None => {}
        }

        for u in self.data.lock().unwrap().uses.clone().iter().rev() {
            u.dump(map);
        }

        let data = self.data.lock().unwrap();
        for (k, v) in data.mapping.iter() {
            map.insert(k.clone(), v.value_type());
        }
    }


    pub fn readonly(&self) {
        self.data.lock().unwrap().is_readonly = true;
    }

    pub fn to_string(&self) -> String {
        let mut map = HashMap::new();
        self.dump(&mut map);
        map.iter().map(|(k, v)| k.clone()).collect::<Vec<String>>().join(", ")
    }
}
