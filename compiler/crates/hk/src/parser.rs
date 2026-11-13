use std::collections::HashSet;
use super::{HkError, Result};
use super::ast::{E,S};
use super::lexer::{K,Lit,Token};

pub(super) struct Parser { t:Vec<Token>, i:usize, name:String, source_text:String, depth:usize }
impl Parser {
    pub(super) fn new(t:Vec<Token>,name:&str,source_text:&str)->Self{Self{t,i:0,name:name.into(),source_text:source_text.into(),depth:0}}
    pub(super) fn parse(&mut self)->Result<Vec<S>>{let mut s=vec![];while !self.check(K::Eof){s.push(self.decl()?);}Ok(s)}
    fn decl(&mut self)->Result<S>{self.with_depth(|p|p.decl_inner())}
    fn decl_inner(&mut self)->Result<S>{
        if self.take(K::Let){return self.var(true);}
        if self.take(K::Const){return self.var(false);}
        if self.take(K::Fn){return self.function();}
        self.stmt()
    }
    fn var(&mut self,m:bool)->Result<S>{
        let n=self.id("Expected variable name.")?;self.expect(K::Eq,"Expected '=' after variable name.")?;
        let e=self.expr()?;self.expect(K::Semi,"Expected ';' after declaration.")?;
        Ok(S::Var(n.text,e,m,n.line,n.col))
    }
    fn function(&mut self)->Result<S>{
        let n=self.id("Expected function name.")?;self.expect(K::LParen,"Expected '(' after function name.")?;
        let mut p=vec![];let mut names=HashSet::new();if !self.check(K::RParen){loop{if p.len()>=255{return Err(self.error("Too many parameters."));}let param=self.id("Expected parameter name.")?;if !names.insert(param.text.clone()){return Err(HkError::new("E2001",format!("Duplicate parameter '{}'.",param.text),&self.name,param.line,param.col,&self.source_text));}p.push(param.text);if !self.take(K::Comma){break;}}}
        self.expect(K::RParen,"Expected ')' after parameters.")?;self.expect(K::LBrace,"Expected '{' before function body.")?;
        let b=self.block_body()?;Ok(S::Fn(n.text,p,b,n.line,n.col))
    }
    fn stmt(&mut self)->Result<S>{self.with_depth(|p|p.stmt_inner())}
    fn stmt_inner(&mut self)->Result<S>{
        if self.take(K::If){self.expect(K::LParen,"Expected '(' after if.")?;let c=self.expr()?;self.expect(K::RParen,"Expected ')' after condition.")?;let a=Box::new(self.body()?);let b=if self.take(K::Else){Some(Box::new(self.body()?))}else{None};return Ok(S::If(c,a,b));}
        if self.take(K::While){self.expect(K::LParen,"Expected '(' after while.")?;let c=self.expr()?;self.expect(K::RParen,"Expected ')' after condition.")?;return Ok(S::While(c,Box::new(self.body()?)));}
        if self.take(K::For){
            self.expect(K::LParen,"Expected '(' after for.")?;
            let init=if self.take(K::Semi){None}else if self.take(K::Let){Some(Box::new(self.var(true)?))}else if self.take(K::Const){Some(Box::new(self.var(false)?))}else{let e=self.expr()?;self.expect(K::Semi,"Expected ';' after initializer.")?;Some(Box::new(S::Expr(e)))};
            let cond=if self.check(K::Semi){None}else{Some(self.expr()?)};self.expect(K::Semi,"Expected ';' after condition.")?;
            let inc=if self.check(K::RParen){None}else{Some(self.expr()?)};self.expect(K::RParen,"Expected ')' after for clauses.")?;
            return Ok(S::For(init,cond,inc,Box::new(self.body()?)));
        }
        if self.take(K::Return){let t=self.prev().clone();let v=if self.check(K::Semi){None}else{Some(self.expr()?)};self.expect(K::Semi,"Expected ';' after return.")?;return Ok(S::Return(v,t.line,t.col));}
        if self.take(K::LBrace){return Ok(S::Block(self.block_body()?));}
        let e=self.expr()?;self.expect(K::Semi,"Expected ';' after expression.")?;Ok(S::Expr(e))
    }
    fn body(&mut self)->Result<S>{if self.take(K::LBrace){Ok(S::Block(self.block_body()?))}else{self.stmt()}}
    fn block_body(&mut self)->Result<Vec<S>>{let mut s=vec![];while !self.check(K::RBrace)&&!self.check(K::Eof){s.push(self.decl()?);}self.expect(K::RBrace,"Expected '}' after block.")?;Ok(s)}
    fn expr(&mut self)->Result<E>{self.with_depth(|p|p.assign())}
    fn assign(&mut self)->Result<E>{self.with_depth(|p|p.assign_inner())}
    fn assign_inner(&mut self)->Result<E>{
        let e=self.or()?;
        if self.take(K::Eq){let t=self.prev().clone();let v=self.assign()?;return match e{
            E::Var(n,_,_)=>Ok(E::Assign(n,Box::new(v),t.line,t.col)),
            E::Index(o,i,l,c)=>Ok(E::SetIndex(o,i,Box::new(v),l,c)),
            _=>Err(HkError::new("E2002","Invalid assignment target.",&self.name,t.line,t.col,&self.source_text))};}
        Ok(e)
    }
    fn or(&mut self)->Result<E>{let mut e=self.and()?;while self.take(K::Or){let t=self.prev().clone();e=E::Binary(Box::new(e),K::Or,Box::new(self.and()?),t.line,t.col);}Ok(e)}
    fn and(&mut self)->Result<E>{let mut e=self.eq()?;while self.take(K::And){let t=self.prev().clone();e=E::Binary(Box::new(e),K::And,Box::new(self.eq()?),t.line,t.col);}Ok(e)}
    fn eq(&mut self)->Result<E>{let mut e=self.cmp()?;while self.take_any(&[K::EqEq,K::BangEq]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.cmp()?),t.line,t.col);}Ok(e)}
    fn cmp(&mut self)->Result<E>{let mut e=self.term()?;while self.take_any(&[K::Gt,K::GtEq,K::Lt,K::LtEq]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.term()?),t.line,t.col);}Ok(e)}
    fn term(&mut self)->Result<E>{let mut e=self.factor()?;while self.take_any(&[K::Plus,K::Minus]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.factor()?),t.line,t.col);}Ok(e)}
    fn factor(&mut self)->Result<E>{let mut e=self.unary()?;while self.take_any(&[K::Star,K::Slash,K::Percent]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.unary()?),t.line,t.col);}Ok(e)}
    fn unary(&mut self)->Result<E>{self.with_depth(|p|p.unary_inner())}
    fn unary_inner(&mut self)->Result<E>{if self.take_any(&[K::Bang,K::Minus]){let t=self.prev().clone();return Ok(E::Unary(t.kind,Box::new(self.unary()?),t.line,t.col));}self.call()}
    fn call(&mut self)->Result<E>{
        let mut e=self.primary()?;
        loop{
            if self.take(K::LParen){let mut a=vec![];if !self.check(K::RParen){loop{if a.len()>=255{return Err(self.error("Too many call arguments."));}a.push(self.expr()?);if !self.take(K::Comma){break;}}}let t=self.expect(K::RParen,"Expected ')' after arguments.")?;e=E::Call(Box::new(e),a,t.line,t.col);}
            else if self.take(K::LBracket){let idx=self.expr()?;let t=self.expect(K::RBracket,"Expected ']' after index.")?;e=E::Index(Box::new(e),Box::new(idx),t.line,t.col);}
            else if self.take(K::Dot){let n=self.id("Expected property name.")?;e=E::Property(Box::new(e),n.text,n.line,n.col);}
            else{break;}
        }Ok(e)
    }
    fn primary(&mut self)->Result<E>{
        let t=self.advance().clone();
        match t.kind{
            K::True=>Ok(E::Bool(true,t.line,t.col)),K::False=>Ok(E::Bool(false,t.line,t.col)),K::Null=>Ok(E::Null(t.line,t.col)),
            K::Number=>if let Some(Lit::Number(n))=t.literal{Ok(E::Lit(Lit::Number(n),t.line,t.col))}else{Err(self.error("Invalid number token."))},
            K::String=>if let Some(Lit::String(s))=t.literal{Ok(E::Lit(Lit::String(s),t.line,t.col))}else{Err(self.error("Invalid string token."))},
            K::Id=>Ok(E::Var(t.text,t.line,t.col)),
            K::LParen=>{let e=self.expr()?;self.expect(K::RParen,"Expected ')' after expression.")?;Ok(E::Group(Box::new(e)))},
            K::LBracket=>{let mut a=vec![];if !self.check(K::RBracket){loop{a.push(self.expr()?);if !self.take(K::Comma){break;}}}self.expect(K::RBracket,"Expected ']' after array.")?;Ok(E::Array(a,t.line,t.col))},
            _=>Err(HkError::new("E2000","Expected expression.",&self.name,t.line,t.col,&self.source_text))
        }
    }
    fn with_depth<T>(&mut self,f:impl FnOnce(&mut Self)->Result<T>)->Result<T>{if self.depth>=128{return Err(self.error("Maximum parser nesting depth (128) exceeded."));}self.depth+=1;let result=f(self);self.depth-=1;result}
    fn id(&mut self,msg:&str)->Result<Token>{if self.check(K::Id){Ok(self.advance().clone())}else{Err(self.error(msg))}}
    fn expect(&mut self,k:K,msg:&str)->Result<Token>{if self.check(k){Ok(self.advance().clone())}else{Err(self.error(msg))}}
    fn take(&mut self,k:K)->bool{if self.check(k){self.advance();true}else{false}}
    fn take_any(&mut self,ks:&[K])->bool{for k in ks{if self.take(*k){return true;}}false}
    fn check(&self,k:K)->bool{self.peek().kind==k}
    fn advance(&mut self)->&Token{let current=self.i;if !self.check(K::Eof){self.i+=1;}&self.t[current]}
    fn peek(&self)->&Token{&self.t[self.i]}
    fn prev(&self)->&Token{&self.t[self.i-1]}
    fn error(&self,msg:&str)->HkError{HkError::new("E2000",msg,&self.name,self.peek().line,self.peek().col,&self.source_text)}
}

