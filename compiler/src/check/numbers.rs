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

    pub(crate) fn numeric_expression(&self, expr: &Expr) -> Option<Expr> {
        self.numeric_name(expr).map(|name| Expr {
            kind: ExprKind::Name(name.into()),
            span: expr.span,
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
