use super::Checker;
use crate::ast::{Expr, ExprKind};

impl Checker {
    pub(crate) fn numeric_name<'a>(&self, expr: &'a Expr) -> Option<&'a str> {
        let (ExprKind::Int(name) | ExprKind::Float(name)) = &expr.kind else {
            return None;
        };
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.values.contains_key(name))
            .then_some(name)
    }

    pub(crate) fn numeric_expression(&self, expr: &Expr) -> Option<Box<Expr>> {
        self.numeric_name(expr).map(|name| {
            Box::new(Expr {
                kind: ExprKind::Name(name.into()),
                span: expr.span,
            })
        })
    }

    pub(crate) fn value_name<'a>(&self, expr: &'a Expr) -> Option<&'a str> {
        if let ExprKind::Name(name) = &expr.kind {
            Some(name)
        } else {
            self.numeric_name(expr)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    pub(crate) fn numeric_bindings_keep_types_scope_and_intrinsic_boundaries() {
        for source in [
            "1:2;x:1+1;{1:3;y:1};z:1",
            "1:1;01:2;0x1:3;0b1:4;1.0:5.0;1e0:6.0",
            "1<uint8>:2;x<uint8>:1;y:1+3",
            "1:true;|1|x:7",
            "1:@\"debug\";(1).print(7)",
            "1<int32>:(2<int32>){->2+3};x:1(4)",
            "1<(int32)->int32>;1<int32>:(2<int32>){->1(2)}",
            "128<int8>:7;x<int8>:-128",
            "x<int8>:-128",
            "1<(int32)->int32>;1<int32>:(2<int32>)'done{|2==0|{'done->0;'done.leave()};->1(2-01)}",
            "1:=2;r:&1;x:*r;1=3;q:&!1;*q=4",
            "1:({->n:=7});(1).n=8;r:(1).&n;x:*r",
            "1<int32[2]>:=[7,8];1[01]=9",
            "1<null><int32>:=null;|1<int32>|x:1~<int32>",
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
        for (source, code) in [
            ("1:2;1:3", "E203"),
            ("1:2;1=3", "E305"),
            ("1:2;r:&!1", "E305"),
            ("1:=2;r:&1;1=3;x:*r", "E302"),
            ("1<uint8>:2;x<int32>:1", "E207"),
            ("1<uint8>:2;x:-1", "E222"),
            ("1:true;x:1+2", "E222"),
            ("1:2;f:(){->1}", "B001"),
            ("1<int8>:7;x<int8>:-129", "E216"),
        ] {
            let errors = crate::compile(source).unwrap_err();
            assert_eq!(errors[0].code, code, "{source}: {errors:?}");
        }
    }
}

#[cfg(test)]
mod required {
    #[test]
    pub(crate) fn numeric_required_scalars_keep_values_types_and_input_gates() {
        for source in [
            "<T>:{1<uint8>:3;n<uint8>:1+2;flag:1==3;-><int32[n]>};v<T>:[0,0,0,0,0]",
            "1:false;<T>:{flag:1||true;copy:1;->flag<>};v<T>:true",
            "1:3;v<int32[1]>:[0,0,0]",
            "128<int8>:7;<T>:{n:-128;->n<>};v<T>:-8",
            "<T>:{1:2;flag:1<({->3});->flag<>};v<T>:true",
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
        for (source, code) in [
            ("1:=2;<T>:{n:1;->n<>}", "E211"),
            ("1<uint8>:2;<T>:{n<int32>:1;->n<>}", "E207"),
            ("1<uint8>:2;<T>:{n:-1;->n<>}", "E222"),
        ] {
            let errors = crate::compile(source).unwrap_err();
            assert_eq!(errors[0].code, code, "{source}: {errors:?}");
        }
    }
}

#[cfg(test)]
mod identities {
    #[test]
    pub(crate) fn numeric_required_records_and_type_values_follow_ordinary_lookup() {
        for source in [
            "1:<uint8>;<T>:1;v<T>:7",
            "<T>:{1:<uint8>;same:1==<uint8>;->1};v<T>:7",
            "<T>:{1:{->n:3};copy:1;-><int32[copy.n]>};v<T>:[0,0,0]",
            "<T>:{1:{->n:3};copy<{n<int32>}>:{->1};-><int32[copy.n]>};v<T>:[0,0,0]",
            "1:({->n:3});<T>:{n:(1).n;-><int32[n]>};v<T>:[0,0,0]",
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
        let source = "1:({->n:=3});<T>:{n:(1).n;->n<>}";
        assert!(crate::compile(source).is_err());
    }
}

#[cfg(test)]
mod lists {
    #[test]
    pub(crate) fn numeric_list_inputs_keep_their_bound_type_and_storage() {
        for source in [
            "1<uint8>:2;v:[1,3];x<uint8>:v[01]",
            "1<uint8>:2;v:[3,1];x<uint8>:v[01]",
            "1<uint8>:2;v<uint8[2]><uint16[2]>:[1,3]",
            "1<float32>:2.5;v:[1,3.5];x<float32>:v[01]",
            "1:=7;v:[1,2];1=8;x:v[01]",
            "1:({->n<uint8>:2});v:[1,{->n<uint8>:3}]",
            "1<int32[2]>:=[7,8];r:&(1[01]);x:*r",
        ] {
            crate::compile(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        }
        for (source, code) in [
            ("1<int32>:2;v<uint8[2]>:[1,3]", "E207"),
            ("1<uint8>:2;v<int32[2]>:[-1,3]", "E222"),
        ] {
            let errors = crate::compile(source).unwrap_err();
            assert_eq!(errors[0].code, code, "{source}: {errors:?}");
        }
    }
}
