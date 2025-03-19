mod backend;
use koopa::ir::{dfg::DataFlowGraph, entities::ValueData, *};
use std::{collections::LinkedList, option, thread::panicking};

use crate::ast::FuncDef;

pub fn compile(prog: Program) -> String {
    let mut inst_list = LinkedList::<String>::new();
    prog.generate_asm(&mut inst_list);
    inst_list.push_back("\n".to_string());
    inst_list
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>()
        .join("\n")
}
trait GenerateAsm {
    fn generate_asm(&self, asm: &mut LinkedList<String>);
}
trait GenerateIns<T> {
    /// #Returns
    /// If it is expr, return Some(String), or it is a stmt and return None.
    fn generate_ins(&self, asm: &mut LinkedList<String>, data: &T) -> Option<String>;
}

impl GenerateAsm for Program {
    fn generate_asm(&self, asm: &mut LinkedList<String>) {
        asm.push_back(String::from(".text"));
        asm.push_back(format!(".globl main"));
        for &func in self.func_layout() {
            let funcdata = self.func(func);
            funcdata.generate_asm(asm);
        }
    }
}

impl GenerateAsm for FunctionData {
    fn generate_asm(&self, asm: &mut LinkedList<String>) {
        let name = if self.name().len() > 1 {
            &self.name()[1..]
        } else {
            panic!("An invalid function name {}", self.name())
        };
        asm.push_back(format!("{}:", name));

        for (&bb, node) in self.layout().bbs() {
            for &inst in node.insts().keys() {
                inst.generate_ins(asm, self.dfg());
            }
        }
    }
}

impl GenerateIns<DataFlowGraph> for Value {
    fn generate_ins(&self, asm: &mut LinkedList<String>, dfg: &DataFlowGraph) -> Option<String> {
        let reg = backend::get_ins_reg(self);
        if reg.is_some()
        {
            return reg;
        }
        
        let value_data = dfg.value(*self);
        match value_data.kind() {
            ValueKind::Return(ins) => {
                let value = ins.value();
                if let Some(value) = value
                {
                    let reg = value.generate_ins(asm, dfg).expect("Expect an expr");
                    asm.push_back(format!("li a0, {}", reg));
                    asm.push_back("ret".to_string());

                }
                else
                {
                    asm.push_back("li a0, 0".to_string());
                    asm.push_back("ret".to_string());
                    
                }
                None
            },

            ValueKind::Binary(ins) => {
                let lhs = ins.lhs().generate_ins(asm, dfg).expect("Expect an expr");
                let rhs = ins.rhs().generate_ins(asm, dfg).expect("Expect an expr");
                let reg: String = backend::alloc_ins_rg(self, Some(rhs.as_str()));


                match ins.op() {
                    BinaryOp::Eq =>
                    {
                        asm.push_back(format!("xor {}, {}, {}", reg, lhs, rhs));
                        asm.push_back(format!("seqz {}, {}", reg, reg));
                        Some(rhs)
                    },

                    BinaryOp::Sub =>
                    {
                        asm.push_back(format!("sub {}, {}, {}", reg, lhs, rhs));
                        Some(rhs)
                    },

                    other => panic!("Not implement binary op {:#?}", other)
                }
            },
            
            ValueKind::Integer(ins) =>
            {
                let val = ins.value();
                if val == 0
                {
                    Some("x0".to_string())
                }
                else {
                    let reg = backend::alloc_ins_rg(self, None);
                    asm.push_back(format!("li {}, {}", reg, val.to_string()));
                    Some(reg)
                }
            }
            other => panic!("Not Implemented for value type {:#?}", other),
        }
    }
}
