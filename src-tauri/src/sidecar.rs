// SPDX-License-Identifier: AGPL-3.0-only
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use serde_json::{json, Value};

pub struct Sidecar {
    _child: Child,
    stdin:  Mutex<ChildStdin>,
    stdout: Mutex<BufReader<ChildStdout>>,
    next_id: Mutex<u64>,
}

impl Sidecar {
    pub fn spawn(exe: &str) -> Result<Self, String> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Gagal menjalankan sidecar: {e}"))?;
        let stdin  = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Ok(Self {
            _child:  child,
            stdin:   Mutex::new(stdin),
            stdout:  Mutex::new(stdout),
            next_id: Mutex::new(1),
        })
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            let id = *n;
            *n += 1;
            id
        };
        let req = json!({"id": id, "method": method, "params": params});
        let mut line = serde_json::to_string(&req).unwrap();
        line.push("
".chars().next().unwrap());
        self.stdin.lock().unwrap().write_all(line.as_bytes())
            .map_err(|e| format!("Gagal kirim ke sidecar: {e}"))?;
        let mut resp_line = String::new();
        self.stdout.lock().unwrap().read_line(&mut resp_line)
            .map_err(|e| format!("Gagal baca dari sidecar: {e}"))?;
        let resp: Value = serde_json::from_str(resp_line.trim())
            .map_err(|e| format!("Respons sidecar tidak valid: {e}"))?;
        if let Some(err) = resp.get("error") {
            return Err(err["message"].as_str().unwrap_or("Error sidecar").to_string());
        }
        Ok(resp["result"].clone())
    }
}
