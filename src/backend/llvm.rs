use std::collections::HashMap;
use inkwell::{
    AddressSpace, IntPredicate, builder::Builder, context::Context, module::Module, targets::{InitializationConfig, Target}, types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum}, values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue}
};

use crate::{ast::*, backend::Backend};
use crate::lexer::{Numeric, Operator, Type};

pub struct LLVMGenerator<'ctx>
{
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    vars: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    out: String,
}

impl <'ctx> LLVMGenerator<'ctx> {
    pub fn new(context: &'ctx Context) -> Self
    {
        Target::initialize_native(&InitializationConfig::default())
            .expect("Unable to initialize the native target!");

        Self {
            context,
            module:  context.create_module("gravi"),
            builder: context.create_builder(),
            vars:    HashMap::new(),
            out:  String::new(),
        }
    }

    fn preprocess(&mut self, prog: &Program)
    {
        for global in prog.items()
        {
            let fun = match global
            {
                Global::Fun(FunKind::Custom(f) | FunKind::Extern(f)) => f,
                _ => continue
            };

            let ptypes: Vec<BasicMetadataTypeEnum<'ctx>> = fun.params.iter()
                .map(|p| self.get_type(p.ty()).into()).collect();

            let ftype = if fun.ret == Type::None
            {
                self.context.void_type().fn_type(&ptypes, false)
            } else {
                self.get_type(fun.ret()).fn_type(&ptypes, false)
            };

            self.module.add_function(fun.identifier(), ftype, None);
        }
    }

    pub fn process(&mut self, prog: &Program)
    {
        self.preprocess(prog);

        for global in prog.items()
        {
            match global {
                Global::Fun(FunKind::Custom(f)) => self.gen_fun(f),
                Global::Fun(FunKind::Entry(f)) => self.gen_entry(f),
                _ => {}
            }
        }

        self.out = self.module.print_to_string().to_string();
    }

    fn get_type(&self, ty: &Type) -> BasicTypeEnum<'ctx>
    {
        match ty
        {
            Type::Numeric(n) => match n {
                Numeric::U8 | Numeric::I8 => self.context.i8_type().into(),
                Numeric::U16 | Numeric::I16 => self.context.i16_type().into(),
                Numeric::U32 | Numeric::I32 => self.context.i32_type().into(),
                Numeric::U64 | Numeric::I64 | Numeric::USize => self.context.i64_type().into(),
                Numeric::F16 => self.context.f16_type().into(),
                Numeric::F32 => self.context.f32_type().into(),
                Numeric::F64 => self.context.f64_type().into()
            },
            Type::Boolean => self.context.bool_type().into(),
            Type::Character => self.context.i8_type().into(),
            Type::StringLiteral => self.context.ptr_type(AddressSpace::default()).into(),
            Type::Custom(_) => self.context.ptr_type(AddressSpace::default()).into(),
            Type::None => unreachable!("void is not available!"),
        }
    }

    fn gen_entry(&mut self, fun: &Function) {
        let ptypes: Vec<BasicMetadataTypeEnum<'ctx>> = fun.params.iter()
                .map(|p| self.get_type(p.ty()).into()).collect();
        let ftype = self.get_type(&Type::Numeric(Numeric::I32)).fn_type(&ptypes, false);
        let func = self.module.add_function(fun.identifier(), ftype, None);
        let entry_bb = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry_bb);

        self.vars.clear();
        self.gen_block(&fun.body);

        if self.builder.get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            let zero = self.context.i32_type().const_int(0, false);
            self.builder.build_return(Some(&zero)).unwrap();
        }
    }

    fn gen_fun(&mut self, fun: &Function)
    {
        let func = self.module.get_function(fun.identifier()).unwrap();

        let entry = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(entry);
        
        self.vars.clear();
        for (i, param) in fun.params().iter().enumerate()
        {
            let ty = self.get_type(param.ty());
            let ptr = self.builder.build_alloca(ty, param.identifier()).unwrap();
            let val = func.get_nth_param(i as u32).unwrap();
            self.builder.build_store(ptr, val).unwrap();
            self.vars.insert(param.id.clone(), (ptr, ty));
        }

        self.gen_block(fun.body());
        
        if fun.ret == Type::None
        {
            if self.builder.get_insert_block().and_then(|b| b.get_terminator()).is_none()
            {
                self.builder.build_return(None).unwrap();
            }
        }
    }

    fn gen_expr(&mut self, expr: &Expr) -> Option<BasicValueEnum<'ctx>>
    {
        let mut res = None;

        match expr
        {
            Expr::Literal(l) => {
                if let Ok(v) = l.parse::<i64>()
                {
                    res = Some(self.context.i32_type().const_int(v as u64, true).into())
                } else if let Ok(v) = l.parse::<f64>() {
                    res = Some(self.context.f64_type().const_float(v).into())
                } else {
                    // error! unknown literal!
                }
            },
            Expr::Binary(b) => res = self.gen_binary(b),
            Expr::CharLiteral(c) => res = Some(self.context.i8_type().const_int(*c as u64, false).into()),
            Expr::StringLiteral(s) => res = Some(self.build_str_ptr(s)),
            Expr::Identifier(name) => {
                let (ptr, ty) = self.vars[name];
                res = Some(self.builder.build_load(ty, ptr, name).unwrap());
            },
            Expr::Grouped(expr) => res = self.gen_expr(expr),
            Expr::Boolean(op) => {
                res = self.gen_comp(op);
            },
            Expr::Unary(u) => {
                let val = self.gen_expr(u.right());

                match u.op {
                    Operator::Sub => match val
                    {
                        Some(BasicValueEnum::IntValue(v)) => res = Some(self.builder.build_int_neg(v, "neg").unwrap().into()),
                        Some(BasicValueEnum::FloatValue(v)) => res = Some(self.builder.build_float_neg(v, "fneg").unwrap().into()),
                        _ => {
                            // error! negation op on a non-numeric value!
                        }
                    },
                    Operator::LNot => match val {
                        Some(BasicValueEnum::IntValue(v)) => res = Some(self.builder.build_not(v, "not").unwrap().into()),
                        _ => {
                            // error! "not" op on a non-boolean value!
                        }
                    },
                    _ => {}
                }
            },
            Expr::Call(name, args) => {
                res = self.gen_call(name, args);
            },
            Expr::Cast(c) => {
                let what = self.gen_val(&c.what).unwrap();
                let to = self.get_type(&c.to);

                match (what, to) {
                    (BasicValueEnum::IntValue(v),   BasicTypeEnum::IntType(t))   =>
                        res = Some(self.builder.build_int_cast(v, t, "icast").unwrap().into()),
                    (BasicValueEnum::IntValue(v),   BasicTypeEnum::FloatType(t)) =>
                        res = Some(self.builder.build_signed_int_to_float(v, t, "itof").unwrap().into()),
                    (BasicValueEnum::FloatValue(v), BasicTypeEnum::IntType(t))   =>
                        res = Some(self.builder.build_float_to_signed_int(v, t, "ftoi").unwrap().into()),
                    (BasicValueEnum::FloatValue(v), BasicTypeEnum::FloatType(t)) =>
                        res = Some(self.builder.build_float_cast(v, t, "fcast").unwrap().into()),
                    _ => {
                        // error! unsupported cast!
                    }
                }
            },
            _ => {}
        }

        res
    }

    fn gen_comp(&mut self, op: &BinaryOp) -> Option<BasicValueEnum<'ctx>>
    {
        use inkwell::IntPredicate::*;
        use inkwell::FloatPredicate;
        
        let mut res = None;

        let left = self.gen_expr(op.left());
        let right = self.gen_expr(op.right());

        if let Some(l) = left && let Some(r) = right
        {
            match (l, r) {
                (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                    let mut pred: Option<IntPredicate> = None;

                    match op.op {
                        Operator::Eq  => pred = Some(EQ),
                        Operator::NEq => pred = Some(NE),
                        Operator::L   => pred = Some(SLT),
                        Operator::LE  => pred = Some(SLE),
                        Operator::G   => pred = Some(SGT),
                        Operator::GE  => pred = Some(SGE),
                        Operator::LAnd => {
                            return Some(self.builder.build_and(l, r, "and").unwrap().into());
                        }
                        Operator::LOr => {
                            return Some(self.builder.build_or(l, r, "or").unwrap().into());
                        }
                        _ => {}
                    };

                    res = Some(self.builder.build_int_compare(pred.unwrap(), l, r, "cmp").unwrap().into());
                },
                (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                    let mut pred: Option<FloatPredicate> = None;

                    match op.op {
                        Operator::Eq  => pred = Some(FloatPredicate::OEQ),
                        Operator::NEq => pred = Some(FloatPredicate::ONE),
                        Operator::L   => pred = Some(FloatPredicate::OLT),
                        Operator::LE  => pred = Some(FloatPredicate::OLE),
                        Operator::G   => pred = Some(FloatPredicate::OGT),
                        Operator::GE  => pred = Some(FloatPredicate::OGE),
                        _ => {}
                    };

                    res = Some(self.builder.build_float_compare(pred.unwrap(), l, r, "fcmp").unwrap().into());
                },
                _ => {}
            }
        }

        res
    }


    fn gen_binary(&mut self, op: &BinaryOp) -> Option<BasicValueEnum<'ctx>>
    {
        let mut res = None;

        let left = self.gen_expr(op.left());
        let right = self.gen_expr(op.right());

        if let Some(l) = left && let Some(r) = right
        {
            match (l, r) {
                (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                    match op.op {
                        Operator::Add => res = Some(self.builder.build_int_add(l, r, "add").unwrap().into()),
                        Operator::Sub => res = Some(self.builder.build_int_sub(l, r, "sub").unwrap().into()),
                        Operator::Mul => res = Some(self.builder.build_int_mul(l, r, "mul").unwrap().into()),
                        Operator::Div => res = Some(self.builder.build_int_signed_div(l, r, "div").unwrap().into()),
                        Operator::Mod => res = Some(self.builder.build_int_signed_rem(l, r, "rem").unwrap().into()),
                        _ => {}
                    }
                },
                (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                    match op.op {
                        Operator::Add => res = Some(self.builder.build_float_add(l, r, "fadd").unwrap().into()),
                        Operator::Sub => res = Some(self.builder.build_float_sub(l, r, "fsub").unwrap().into()),
                        Operator::Mul => res = Some(self.builder.build_float_mul(l, r, "fmul").unwrap().into()),
                        Operator::Div => res = Some(self.builder.build_float_div(l, r, "fdiv").unwrap().into()),
                        _ => {}
                    }
                },
                _ => {}
            }
        }

        res
    }

    fn gen_var(&mut self, var: &Var)
    {
        match var
        {
            Var::Decl(v) => {
                let ty = self.get_type(v.ty());
                let ptr = self.builder.build_alloca(ty, v.identifier()).unwrap();

                if let Some(val) = v.value()
                {
                    let init = self.gen_val(val).unwrap();
                    self.builder.build_store(ptr, init).unwrap();
                }

                self.vars.insert(v.id.clone(), (ptr, ty));
            },
            Var::Var(v) => {
                if let Some(val) = &v.val
                {
                    let nval = self.gen_val(val).unwrap();
                    let (ptr, _) = self.vars[&v.name];
                    self.builder.build_store(ptr, nval).unwrap();
                }
            },
        }
    }

    fn gen_ret(&mut self, val: &Value)
    {
        if matches!(val, Value::Null)
        {
            self.builder.build_return(None).unwrap();
        } else {
            let v = self.gen_val(val).unwrap();
            self.builder.build_return(Some(&v)).unwrap();
        }
    }

    fn gen_block(&mut self, items: &[Items])
    {
        for item in items
        {
            match item {
                Items::Var(var) => self.gen_var(var),
                Items::Ret(val) => self.gen_ret(val),
                Items::Expr(val) => {
                    self.gen_val(val);
                },
                _ => {}
            }
        }
    }

    fn build_str_ptr(&self, s: &str) -> BasicValueEnum<'ctx> {
        self.builder.build_global_string_ptr(s, "str").unwrap().as_pointer_value().into()
    }

    fn gen_val(&mut self, val: &Value) -> Option<BasicValueEnum<'ctx>>
    {
        let mut res = None;
        
        match val
        {
            Value::Expression(expr) => res = self.gen_expr(expr),
            Value::StringLiteral(s) => res = Some(self.build_str_ptr(s)),
            Value::Char(c) => res = Some(self.context.i8_type().const_int(*c as u64, false).into()),
            Value::Boolean(b) => res = Some(self.context.bool_type().const_int(
                                                    if b == &BoolValue::True { 1 } else { 0 }, false).into()),
            Value::Call(name, args) => res = self.gen_call(name, args),
            Value::List(_) => {},
            Value::Block(_, items) => self.gen_block(items),
            Value::IfElse(ifelse) => self.gen_if(ifelse),
            Value::Loop(l) => self.gen_loop(l),
            Value::Null => {},
        }

        res
    }

    fn gen_call(&mut self, name: &str, args: &[Value]) -> Option<BasicValueEnum<'ctx>>
    {
        let res: Option<BasicValueEnum<'ctx>>;

        if name == "show"
        {
            self.gen_show(args);
            return None;
        }

        let func = self.module.get_function(name).unwrap();
        let compiled: Vec<BasicMetadataValueEnum<'ctx>> = args.iter()
            .filter_map(|a| self.gen_val(a))
            .map(|v| v.into()).collect();

        let call = self.builder.build_call(func, &compiled, name).unwrap();
        res = call.try_as_basic_value().basic();

        res
    }

    fn gen_if(&mut self, ifelse: &IfElse)
    {
        let parent = self.builder.get_insert_block().unwrap().get_parent().unwrap();

        let then_bb = self.context.append_basic_block(parent, "then");
        let else_bb = self.context.append_basic_block(parent, "else");
        let merge_bb = self.context.append_basic_block(parent, "ifmerge");
    
        let cond = self.gen_val(ifelse.condition().as_ref().unwrap()).unwrap();
        let cond_int = cond.into_int_value();
        self.builder.build_conditional_branch(cond_int, then_bb, else_bb).unwrap();

        self.builder.position_at_end(then_bb);
        self.gen_block(ifelse.body());
        if self.builder.get_insert_block().unwrap().get_terminator().is_none()
        {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(else_bb);
        if let Some(elif) = ifelse.else_if()
        {
            if elif.condition().is_some()
            {
                self.gen_if(elif);
            } else {
                self.gen_block(elif.body());
            }
        }
        if self.builder.get_insert_block().unwrap().get_terminator().is_none()
        {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(merge_bb);
    }

    fn gen_loop(&mut self, l: &Loop)
    {
        let parent = self.builder.get_insert_block().unwrap().get_parent().unwrap();

        match &l.cond {
            None => {
                let loop_bb = self.context.append_basic_block(parent, "loop");
                let exit_bb = self.context.append_basic_block(parent, "loopexitinf");

                self.builder.build_unconditional_branch(loop_bb).unwrap();
                self.builder.position_at_end(loop_bb);

                self.gen_block(&l.body);

                if self.builder.get_insert_block().unwrap().get_terminator().is_none()
                {
                    self.builder.build_unconditional_branch(loop_bb).unwrap();
                }
                self.builder.position_at_end(exit_bb);
            },
            Some(cond) => match cond.value()
            {
                Some(Value::Expression(Expr::Range(rng))) => {
                    let hdr_bb = self.context.append_basic_block(parent, "forhdr");
                    let bd_bb = self.context.append_basic_block(parent, "forbody");
                    let exit_bb = self.context.append_basic_block(parent, "forexit");
                
                    let idx_ty = self.context.i64_type();
                    let idx_ptr = self.builder.build_alloca(idx_ty, cond.identifier()).unwrap();
                    self.vars.insert(cond.identifier().to_owned(), (idx_ptr, idx_ty.into()));

                    let start_val = self.gen_expr(rng.start()).unwrap().into_int_value();
                    self.builder.build_store(idx_ptr, start_val).unwrap();
                    self.builder.build_unconditional_branch(hdr_bb).unwrap();

                    self.builder.position_at_end(hdr_bb);
                    let idx_val = self.builder.build_load(idx_ty, idx_ptr, "i").unwrap().into_int_value();
                    let end_val = self.gen_expr(rng.end()).unwrap().into_int_value();

                    let cmp = if rng.inclusive()
                    {
                        self.builder.build_int_compare(IntPredicate::SLE, idx_val, end_val, "cmp")
                    } else {
                        self.builder.build_int_compare(IntPredicate::SLT, idx_val, end_val, "cmp")
                    }.unwrap();
                    self.builder.build_conditional_branch(cmp, bd_bb, exit_bb).unwrap();
                
                    self.builder.position_at_end(bd_bb);
                    self.gen_block(&l.body);
                    let idx_cur = self.builder.build_load(idx_ty, idx_ptr, "i").unwrap().into_int_value();
                    let one = idx_ty.const_int(1, false);
                    let idx_inc = self.builder.build_int_add(idx_cur, one, "inc").unwrap();
                    self.builder.build_store(idx_ptr, idx_inc).unwrap();
                    if self.builder.get_insert_block().unwrap().get_terminator().is_none()
                    {
                        self.builder.build_unconditional_branch(hdr_bb).unwrap();
                    }
                    
                    self.builder.position_at_end(exit_bb);
                },
                Some(Value::Expression(expr @ Expr::Boolean(_))) => {
                    let hdr_bb = self.context.append_basic_block(parent, "whilehdr");
                    let bd_bb = self.context.append_basic_block(parent, "whilebody");
                    let exit_bb = self.context.append_basic_block(parent, "whileexit");
                    
                    self.builder.build_unconditional_branch(hdr_bb).unwrap();

                    self.builder.position_at_end(hdr_bb);
                    let cond = self.gen_expr(expr).unwrap().into_int_value();
                    self.builder.build_conditional_branch(cond, bd_bb, exit_bb).unwrap();

                    self.builder.position_at_end(bd_bb);
                    self.gen_block(&l.body);
                    if self.builder.get_insert_block().unwrap().get_terminator().is_none()
                    {
                        self.builder.build_unconditional_branch(hdr_bb).unwrap();
                    }
                
                    self.builder.position_at_end(exit_bb);
                },
                _ => {}
            },
        }
    }

    fn decl_show(&self) -> FunctionValue<'ctx>
    {
        if let Some(f) = self.module.get_function("printf") { return f; };

        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let print_ty = self.context.i32_type().fn_type(&[ptr_ty.into()], true);
        self.module.add_function("printf", print_ty, None)
    }

    fn gen_show(&mut self, args: &[Value])
    {
        let show = self.decl_show();

        for arg in args
        {
            let val = match self.gen_val(arg) {
                Some(v) => v,
                None => continue
            };

            match val {
                BasicValueEnum::IntValue(v) if v.get_type().get_bit_width() == 1 => {
                    let true_v = self.build_str_ptr("true\n");
                    let false_v = self.build_str_ptr("false\n");
                    let fmt = self.builder.build_select(v, true_v, false_v, "boolstr").unwrap();

                    self.builder.build_call(show, &[fmt.into()], "").unwrap();
                },
                BasicValueEnum::IntValue(v) if v.get_type().get_bit_width() == 8 => {
                    let fmt = self.build_str_ptr("%c\n");
                    self.builder.build_call(show, &[fmt.into(), v.into()], "").unwrap();
                },
                BasicValueEnum::IntValue(v) if v.get_type().get_bit_width() == 32 => {
                    let fmt = self.build_str_ptr("%d\n");
                    self.builder.build_call(show, &[fmt.into(), v.into()], "").unwrap();
                },
                BasicValueEnum::IntValue(v) => {
                    let fmt = self.build_str_ptr("%lld\n");
                    self.builder.build_call(show, &[fmt.into(), v.into()], "").unwrap();
                },
                BasicValueEnum::FloatValue(v) => {
                    let f64_v = if v.get_type().get_bit_width() == 32
                    {
                        self.builder.build_float_cast(v, self.context.f64_type(), "f2d").unwrap()
                    } else {
                        v
                    };

                    let fmt = self.build_str_ptr("%g\n");

                    self.builder.build_call(show, &[fmt.into(), f64_v.into()], "").unwrap();
                },
                BasicValueEnum::PointerValue(v) => {
                    let fmt = self.build_str_ptr("%s\n");

                    self.builder.build_call(show, &[fmt.into(), v.into()], "").unwrap();
                }
                _ => {}
            }
        }
    }
}

impl Backend for LLVMGenerator<'_> {
    fn output(&self) -> &String {
        &self.out
    }
}