use super::lexer::{K, Lit};

#[derive(Clone, Debug)]
pub(super) enum E {
    Lit(Lit,usize,usize), Bool(bool,usize,usize), Null(usize,usize), Var(String,usize,usize),
    Assign(String,Box<E>,usize,usize), Unary(K,Box<E>,usize,usize), Binary(Box<E>,K,Box<E>,usize,usize),
    Group(Box<E>), Call(Box<E>,Vec<E>,usize,usize), Array(Vec<E>,usize,usize),
    Index(Box<E>,Box<E>,usize,usize), SetIndex(Box<E>,Box<E>,Box<E>,usize,usize), Property(Box<E>,String,usize,usize),
}
impl E {
    pub(super) fn source_position(&self)->(usize,usize) {
        match self {
            E::Lit(_,l,c)|E::Bool(_,l,c)|E::Var(_,l,c)|E::Assign(_,_,l,c)|
            E::Unary(_,_,l,c)|E::Binary(_,_,_,l,c)|E::Call(_,_,l,c)|
            E::Array(_,l,c)|E::Index(_,_,l,c)|E::SetIndex(_,_,_,l,c)|
            E::Property(_,_,l,c) => (*l,*c),
            E::Null(l,c) => (*l,*c),
            E::Group(inner) => inner.source_position(),
        }
    }
}
#[derive(Clone, Debug)]
pub(super) enum S {
    Var(String,E,bool,usize,usize), Fn(String,Vec<String>,Vec<S>,usize,usize), Expr(E),
    Block(Vec<S>), If(E,Box<S>,Option<Box<S>>), While(E,Box<S>),
    For(Option<Box<S>>,Option<E>,Option<E>,Box<S>), Return(Option<E>,usize,usize),
}

impl S {
    pub(super) fn source_position(&self)->(usize,usize){
        match self {
            S::Var(_,_,_,line,column)|S::Fn(_,_,_,line,column)|S::Return(_,line,column)=>(*line,*column),
            S::Expr(expression)=>expression.source_position(),
            S::If(condition,_,_)|S::While(condition,_)=>condition.source_position(),
            S::For(Some(initializer),_,_,_)=>initializer.source_position(),
            S::For(None,Some(condition),_,_)=>condition.source_position(),
            S::For(_,_,_,body)=>body.source_position(),
            S::Block(statements)=>statements.first().map(S::source_position).unwrap_or((1,1)),
        }
    }
}
