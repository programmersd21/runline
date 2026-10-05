use crate::gh::{JobId, RawLog, RunDetail, RunId};
use std::collections::HashMap;

pub struct Cache {
    pub enabled: bool,
    pub run_details: HashMap<RunId, RunDetail>,
    pub logs: HashMap<JobId, RawLog>,
}

impl Cache {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            run_details: HashMap::new(),
            logs: HashMap::new(),
        }
    }

    pub fn get_run_detail(&self, id: RunId) -> Option<&RunDetail> {
        if self.enabled {
            self.run_details.get(&id)
        } else {
            None
        }
    }

    pub fn insert_run_detail(&mut self, id: RunId, detail: RunDetail) {
        if self.enabled {
            self.run_details.insert(id, detail);
        }
    }

    pub fn get_log(&self, job: JobId) -> Option<&RawLog> {
        if self.enabled {
            self.logs.get(&job)
        } else {
            None
        }
    }

    pub fn insert_log(&mut self, job: JobId, log: RawLog) {
        if self.enabled {
            self.logs.insert(job, log);
        }
    }
}
