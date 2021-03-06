use super::*;
use std::{fs, env};

/// 
#[derive(Debug)]
pub struct Context<'a> {
    spec: &'a CallMsg,
    pub(crate) inputs: Vec<DataInstance<'a>>,
    pub(crate) outputs: Vec<Output<'a>>,
    /// Task attributes
    pub(crate) attributes: Attributes,
    /// Absolute path to task working dir
    pub(crate) work_dir: PathBuf,
    /// Absolute path to staging dir with input and output objects
    stage_dir: PathBuf,
    pub(crate) success: bool,
}

impl<'a> Context<'a> {
    pub(crate) fn for_call_msg(cm: &'a CallMsg, work_dir: &Path) -> Result<Self> {
        assert!(work_dir.is_absolute());
        let stage_dir = work_dir.join("stage");
        fs::create_dir_all(&stage_dir)?;
        let inputs = cm.inputs.iter().enumerate().map(|(order, inp)| {
            DataInstance::new(inp, &stage_dir, order)
        }).collect();
        let outputs = cm.outputs.iter().enumerate().map(|(order, outp)| {
            Output::new(outp, &stage_dir, order)
        }).collect();
        Ok(Context {
            spec: cm,
            inputs: inputs,
            outputs: outputs,
            attributes: Attributes::new(),
            work_dir: work_dir.into(),
            stage_dir: stage_dir,
            success: true,
        })
    }

    pub(crate) fn into_result_msg(self) -> ResultMsg {
        ResultMsg {
            task: self.spec.task,
            success: self.success,
            attributes: self.attributes,
            outputs: self.outputs.into_iter().map(|o| {
                let (os, _cached) = o.into_output_spec();
                os
                }).collect(),
            cached_objects: Vec::new(),
        }
    }
}
