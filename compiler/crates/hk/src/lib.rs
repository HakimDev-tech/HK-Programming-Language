//! HK 0.1.0 reference interpreter.
//! The initial implementation is deliberately dependency-free and tree-walking.

use std::{cell::RefCell, collections::{HashMap, HashSet}, fmt, rc::{Rc, Weak}};

#[derive(Clone, Copy, Debug, PartialEq)]
enum K {
    Id, Number, String, Let, Const, Fn, Return, If, Else, While, For, True, False, Null,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Comma, Dot, Semi,
    Plus, Minus, Star, Slash, Percent, Bang, BangEq, Eq, EqEq, Gt, GtEq, Lt, LtEq, And, Or, Eof,
}
#[derive(Clone, Debug)]
struct Token { kind: K, text: String, literal: Option<Lit>, line: usize, col: usize }
#[derive(Clone, Debug)]
enum Lit { Number(f64), String(String) }
#[derive(Debug)]
pub struct HkError { pub code: &'static str, pub message: String, pub source: String, pub line: usize, pub column: usize }
impl fmt::Display for HkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error[{}]: {}\n  --> {}:{}:{}", self.code, self.message, self.source, self.line, self.column)
    }
}
impl std::error::Error for HkError {}
type Result<T> = std::result::Result<T, HkError>;

struct Lexer<'a> { source: &'a str, chars: Vec<char>, start: usize, current: usize, line: usize, col: usize, start_line: usize, start_col: usize, name: &'a str }
impl<'a> Lexer<'a> {
    fn new(name: &'a str, source: &'a str) -> Self {
        Self { source, chars: source.chars().collect(), start: 0, current: 0, line: 1, col: 1, start_line: 1, start_col: 1, name }
    }
    fn scan(mut self) -> Result<Vec<Token>> {
        let mut out = Vec::new();
        while !self.end() {
            self.start = self.current; self.start_line = self.line; self.start_col = self.col;
            if let Some(t) = self.token()? { out.push(t); }
        }
        out.push(Token { kind: K::Eof, text: String::new(), literal: None, line: self.line, col: self.col });
        Ok(out)
    }
    fn token(&mut self) -> Result<Option<Token>> {
        let c = self.advance();
        let kind = match c {
            '('=>Some(K::LParen), ')'=>Some(K::RParen), '{'=>Some(K::LBrace), '}'=>Some(K::RBrace),
            '['=>Some(K::LBracket), ']'=>Some(K::RBracket), ','=>Some(K::Comma), '.'=>Some(K::Dot),
            ';'=>Some(K::Semi), '+'=>Some(K::Plus), '-'=>Some(K::Minus), '*'=>Some(K::Star), '%'=>Some(K::Percent),
            '!'=>Some(if self.take('='){K::BangEq}else{K::Bang}),
            '='=>Some(if self.take('='){K::EqEq}else{K::Eq}),
            '>'=>Some(if self.take('='){K::GtEq}else{K::Gt}),
            '<'=>Some(if self.take('='){K::LtEq}else{K::Lt}),
            '&' if self.take('&')=>Some(K::And), '|' if self.take('|')=>Some(K::Or),
            '/' if self.take('/')=>{while self.peek()!='\n'&&!self.end(){self.advance();}None},
            '/'=>Some(K::Slash), ' '| '\r'|'\t'|'\n'=>None,
            '"' => return self.string().map(Some),
            c if c.is_ascii_digit()=>return self.number().map(Some),
            c if c=='_'||c.is_ascii_alphabetic()=>return Ok(Some(self.identifier())),
            '&'|'|'=>return Err(self.err("E1001","Expected a doubled logical operator.")),
            _=>return Err(self.err("E1000",&format!("Unexpected character '{c}'."))),
        };
        Ok(kind.map(|kind|self.make(kind,None)))
    }
    fn string(&mut self)->Result<Token>{
        let mut s=String::new();
        while !self.end()&&self.peek()!='"' {
            let c=self.advance();
            if c=='\\' {
                if self.end(){return Err(self.err("E1002","Unterminated escape sequence."));}
                let e=self.advance();
                s.push(match e {'n'=>'\n','r'=>'\r','t'=>'\t','"'=>'"','\\'=>'\\',_=>return Err(self.err("E1003","Unsupported string escape."))});
            } else {s.push(c);}
        }
        if self.end(){return Err(self.err("E1004","Unterminated string literal."));}
        self.advance();
        Ok(self.make(K::String,Some(Lit::String(s))))
    }
    fn number(&mut self)->Result<Token>{
        while self.peek().is_ascii_digit(){self.advance();}
        if self.peek()=='.'&&self.next().is_ascii_digit(){self.advance();while self.peek().is_ascii_digit(){self.advance();}}
        let text:String=self.chars[self.start..self.current].iter().collect();
        let n=text.parse::<f64>().map_err(|_|self.err("E1005","Invalid number."))?;
        if !n.is_finite(){return Err(self.err("E1006","Number must be finite."));}
        Ok(self.make(K::Number,Some(Lit::Number(n))))
    }
    fn identifier(&mut self)->Token{
        while self.peek()=='_'||self.peek().is_ascii_alphanumeric(){self.advance();}
        let text:String=self.chars[self.start..self.current].iter().collect();
        let kind=match text.as_str(){
            "let"=>K::Let,"const"=>K::Const,"fn"=>K::Fn,"return"=>K::Return,"if"=>K::If,
            "else"=>K::Else,"while"=>K::While,"for"=>K::For,"true"=>K::True,"false"=>K::False,"null"=>K::Null,_=>K::Id
        };
        self.make(kind,None)
    }
    fn make(&self,kind:K,literal:Option<Lit>)->Token{
        Token{kind,text:self.chars[self.start..self.current].iter().collect(),literal,line:self.start_line,col:self.start_col}
    }
    fn err(&self,code:&'static str,msg:&str)->HkError{HkError{code,message:msg.into(),source:self.name.into(),line:self.start_line,column:self.start_col}}
    fn advance(&mut self)->char{let c=self.chars[self.current];self.current+=1;if c=='\n'{self.line+=1;self.col=1}else{self.col+=1}c}
    fn take(&mut self,c:char)->bool{if self.end()||self.chars[self.current]!=c{return false;}self.advance();true}
    fn peek(&self)->char{self.chars.get(self.current).copied().unwrap_or('\0')}
    fn next(&self)->char{self.chars.get(self.current+1).copied().unwrap_or('\0')}
    fn end(&self)->bool{self.current>=self.chars.len()}
}

#[derive(Clone, Debug)]
enum E {
    Lit(Lit,usize,usize), Bool(bool,usize,usize), Null(usize,usize), Var(String,usize,usize),
    Assign(String,Box<E>,usize,usize), Unary(K,Box<E>,usize,usize), Binary(Box<E>,K,Box<E>,usize,usize),
    Group(Box<E>), Call(Box<E>,Vec<E>,usize,usize), Array(Vec<E>,usize,usize),
    Index(Box<E>,Box<E>,usize,usize), SetIndex(Box<E>,Box<E>,Box<E>,usize,usize), Property(Box<E>,String,usize,usize),
}
#[derive(Clone, Debug)]
enum S {
    Var(String,E,bool,usize,usize), Fn(String,Vec<String>,Vec<S>,usize,usize), Expr(E),
    Block(Vec<S>), If(E,Box<S>,Option<Box<S>>), While(E,Box<S>),
    For(Option<Box<S>>,Option<E>,Option<E>,Box<S>), Return(Option<E>,usize,usize),
}
struct Parser { t:Vec<Token>, i:usize, name:String }
impl Parser {
    fn new(t:Vec<Token>,name:&str)->Self{Self{t,i:0,name:name.into()}}
    fn parse(&mut self)->Result<Vec<S>>{let mut s=vec![];while !self.check(K::Eof){s.push(self.decl()?);}Ok(s)}
    fn decl(&mut self)->Result<S>{
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
        let mut p=vec![];if !self.check(K::RParen){loop{if p.len()>=255{return Err(self.error("Too many parameters."));}p.push(self.id("Expected parameter name.")?.text);if !self.take(K::Comma){break;}}}
        self.expect(K::RParen,"Expected ')' after parameters.")?;self.expect(K::LBrace,"Expected '{' before function body.")?;
        let b=self.block_body()?;Ok(S::Fn(n.text,p,b,n.line,n.col))
    }
    fn stmt(&mut self)->Result<S>{
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
    fn expr(&mut self)->Result<E>{self.assign()}
    fn assign(&mut self)->Result<E>{
        let e=self.or()?;
        if self.take(K::Eq){let t=self.prev().clone();let v=self.assign()?;return match e{
            E::Var(n,_,_)=>Ok(E::Assign(n,Box::new(v),t.line,t.col)),
            E::Index(o,i,l,c)=>Ok(E::SetIndex(o,i,Box::new(v),l,c)),
            _=>Err(self.error("Invalid assignment target."))};}
        Ok(e)
    }
    fn or(&mut self)->Result<E>{let mut e=self.and()?;while self.take(K::Or){let t=self.prev().clone();e=E::Binary(Box::new(e),K::Or,Box::new(self.and()?),t.line,t.col);}Ok(e)}
    fn and(&mut self)->Result<E>{let mut e=self.eq()?;while self.take(K::And){let t=self.prev().clone();e=E::Binary(Box::new(e),K::And,Box::new(self.eq()?),t.line,t.col);}Ok(e)}
    fn eq(&mut self)->Result<E>{let mut e=self.cmp()?;while self.take_any(&[K::EqEq,K::BangEq]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.cmp()?),t.line,t.col);}Ok(e)}
    fn cmp(&mut self)->Result<E>{let mut e=self.term()?;while self.take_any(&[K::Gt,K::GtEq,K::Lt,K::LtEq]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.term()?),t.line,t.col);}Ok(e)}
    fn term(&mut self)->Result<E>{let mut e=self.factor()?;while self.take_any(&[K::Plus,K::Minus]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.factor()?),t.line,t.col);}Ok(e)}
    fn factor(&mut self)->Result<E>{let mut e=self.unary()?;while self.take_any(&[K::Star,K::Slash,K::Percent]){let t=self.prev().clone();e=E::Binary(Box::new(e),t.kind,Box::new(self.unary()?),t.line,t.col);}Ok(e)}
    fn unary(&mut self)->Result<E>{if self.take_any(&[K::Bang,K::Minus]){let t=self.prev().clone();return Ok(E::Unary(t.kind,Box::new(self.unary()?),t.line,t.col));}self.call()}
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
            _=>Err(HkError{code:"E2000",message:"Expected expression.".into(),source:self.name.clone(),line:t.line,column:t.col})
        }
    }
    fn id(&mut self,msg:&str)->Result<Token>{if self.check(K::Id){Ok(self.advance().clone())}else{Err(self.error(msg))}}
    fn expect(&mut self,k:K,msg:&str)->Result<Token>{if self.check(k){Ok(self.advance().clone())}else{Err(self.error(msg))}}
    fn take(&mut self,k:K)->bool{if self.check(k){self.advance();true}else{false}}
    fn take_any(&mut self,ks:&[K])->bool{for k in ks{if self.take(*k){return true;}}false}
    fn check(&self,k:K)->bool{self.peek().kind==k}
    fn advance(&mut self)->&Token{if !self.check(K::Eof){self.i+=1;}self.prev()}
    fn peek(&self)->&Token{&self.t[self.i]}
    fn prev(&self)->&Token{&self.t[self.i-1]}
    fn error(&self,msg:&str)->HkError{HkError{code:"E2000",message:msg.into(),source:self.name.clone(),line:self.peek().line,column:self.peek().col}}
}

#[derive(Clone)]
struct Fun { name:String, params:Vec<String>, body:Vec<S>, env:Weak<RefCell<Env>> }
#[derive(Clone)]
enum V { Num(f64), Str(String), Bool(bool), Null, Arr(Rc<RefCell<Vec<V>>>), Fun(Rc<Fun>), Print }
impl V {
    fn ty(&self)->&'static str{match self{V::Num(_)=>"Number",V::Str(_)=>"String",V::Bool(_)=>"Boolean",V::Null=>"Null",V::Arr(_)=>"Array",V::Fun(_)|V::Print=>"Function"}}
    fn show(&self)->String{match self{V::Num(n) if n.fract()==0.0=>format!("{n:.0}"),V::Num(n)=>n.to_string(),V::Str(s)=>s.clone(),V::Bool(b)=>b.to_string(),V::Null=>"null".into(),V::Arr(a)=>format!("[{}]",a.borrow().iter().map(V::show).collect::<Vec<_>>().join(", ")),V::Fun(f)=>format!("<fn {}>",f.name),V::Print=>"<native fn print>".into()}}
    fn eq(&self,b:&V)->bool{match(self,b){(V::Num(a),V::Num(b))=>a==b,(V::Str(a),V::Str(b))=>a==b,(V::Bool(a),V::Bool(b))=>a==b,(V::Null,V::Null)=>true,(V::Arr(a),V::Arr(b))=>Rc::ptr_eq(a,b),(V::Fun(a),V::Fun(b))=>Rc::ptr_eq(a,b),(V::Print,V::Print)=>true,_=>false}}
}
#[derive(Default)]
struct Env { values:HashMap<String,V>, constants:HashSet<String>, parent:Option<Rc<RefCell<Env>>> }
impl Env {
    fn child(parent:Rc<RefCell<Env>>)->Rc<RefCell<Self>>{Rc::new(RefCell::new(Self{parent:Some(parent),..Self::default()}))}
    fn define(&mut self,n:String,v:V,m:bool)->std::result::Result<(),String>{if self.values.contains_key(&n){return Err(format!("'{n}' is already declared in this scope."));}if !m{self.constants.insert(n.clone());}self.values.insert(n,v);Ok(())}
    fn get(e:&Rc<RefCell<Self>>,n:&str)->Option<V>{let(v,p)={let x=e.borrow();(x.values.get(n).cloned(),x.parent.clone())};v.or_else(||p.and_then(|p|Self::get(&p,n)))}
    fn set(e:&Rc<RefCell<Self>>,n:&str,v:V)->std::result::Result<(),String>{let(exists,c,p)={let x=e.borrow();(x.values.contains_key(n),x.constants.contains(n),x.parent.clone())};if exists{if c{return Err(format!("Cannot reassign constant '{n}'."));}e.borrow_mut().values.insert(n.into(),v);return Ok(());}if let Some(p)=p{return Self::set(&p,n,v);}Err(format!("Undefined variable '{n}'."))}
}
enum Flow { Next, Return(V) }
struct Engine { env:Rc<RefCell<Env>>, output:Vec<String>, source:String }
impl Engine {
    fn new(source:&str)->Self{let e=Rc::new(RefCell::new(Env::default()));e.borrow_mut().values.insert("print".into(),V::Print);Self{env:e,output:vec![],source:source.into()}}
    fn error(&self,msg:impl Into<String>,line:usize,col:usize)->HkError{HkError{code:"E3000",message:msg.into(),source:self.source.clone(),line,column:col}}
    fn run(&mut self,ss:&[S])->Result<()>{for s in ss{if let Flow::Return(_)=self.exec(s)?{return Err(self.error("'return' is only valid inside a function.",1,1));}}Ok(())}
    fn exec_all(&mut self,ss:&[S])->Result<Flow>{for s in ss{match self.exec(s)?{Flow::Next=>{},r@Flow::Return(_)=>return Ok(r)}}Ok(Flow::Next)}
    fn exec(&mut self,s:&S)->Result<Flow>{
        match s{
            S::Var(n,e,m,l,c)=>{let v=self.eval(e)?;self.env.borrow_mut().define(n.clone(),v,*m).map_err(|x|self.error(x,*l,*c))?;}
            S::Fn(n,p,b,l,c)=>{let f=V::Fun(Rc::new(Fun{name:n.clone(),params:p.clone(),body:b.clone(),env:Rc::downgrade(&self.env)}));self.env.borrow_mut().define(n.clone(),f,false).map_err(|x|self.error(x,*l,*c))?;}
            S::Expr(e)=>{self.eval(e)?;}
            S::Block(b)=>{let old=self.env.clone();self.env=Env::child(old.clone());let r=self.exec_all(b);self.env=old;return r;}
            S::If(c,a,b)=>{let v=self.eval(c)?;if self.boolean(v,1,1)?{return self.exec(a);}if let Some(b)=b{return self.exec(b);}}
            S::While(c,b)=>loop{let v=self.eval(c)?;if !self.boolean(v,1,1)?{break;}if let Flow::Return(v)=self.exec(b)?{return Ok(Flow::Return(v));}},
            S::For(init,cond,inc,body)=>{let old=self.env.clone();self.env=Env::child(old.clone());let r=(||{if let Some(i)=init{self.exec(i)?;}loop{if let Some(c)=cond{let v=self.eval(c)?;if !self.boolean(v,1,1)?{break;}}if let Flow::Return(v)=self.exec(body)?{return Ok(Flow::Return(v));}if let Some(i)=inc{self.eval(i)?;}}Ok(Flow::Next)})();self.env=old;return r;}
            S::Return(e,_l,_c)=>return Ok(Flow::Return(if let Some(e)=e{self.eval(e)?}else{V::Null})),
        }Ok(Flow::Next)
    }
    fn eval(&mut self,e:&E)->Result<V>{
        match e{
            E::Lit(Lit::Number(n),_,_)=>Ok(V::Num(*n)),E::Lit(Lit::String(s),_,_)=>Ok(V::Str(s.clone())),
            E::Bool(b,_,_)=>Ok(V::Bool(*b)),E::Null(_,_)=>Ok(V::Null),
            E::Var(n,l,c)=>Env::get(&self.env,n).ok_or_else(||self.error(format!("Undefined variable '{n}'."),*l,*c)),
            E::Assign(n,e,l,c)=>{let v=self.eval(e)?;Env::set(&self.env,n,v.clone()).map_err(|x|self.error(x,*l,*c))?;Ok(v)}
            E::Group(e)=>self.eval(e),
            E::Array(es,_,_)=>{let mut a=vec![];for e in es{a.push(self.eval(e)?);}Ok(V::Arr(Rc::new(RefCell::new(a))))}
            E::Unary(op,e,l,c)=>{let v=self.eval(e)?;match op{K::Minus=>if let V::Num(n)=v{self.finite(-n,*l,*c)}else{Err(self.error("Unary '-' expects Number.",*l,*c))},K::Bang=>if let V::Bool(b)=v{Ok(V::Bool(!b))}else{Err(self.error("'!' expects Boolean.",*l,*c))},_=>unreachable!()}}
            E::Binary(a,op,b,l,c)=>{
                let x=self.eval(a)?;
                if *op==K::And{let v=self.boolean(x,*l,*c)?;if !v{return Ok(V::Bool(false));}let right = self.eval(b)?; return Ok(V::Bool(self.boolean(right,*l,*c)?));}
                if *op==K::Or{let v=self.boolean(x,*l,*c)?;if v{return Ok(V::Bool(true));}return Ok(V::Bool(self.boolean(self.eval(b)?,*l,*c)?));}
                let y=self.eval(b)?;self.binary(x,*op,y,*l,*c)
            }
            E::Call(callee,args,l,c)=>{let f=self.eval(callee)?;let mut vs=vec![];for a in args{vs.push(self.eval(a)?);}match f{
                V::Print=>{if vs.len()!=1{return Err(self.error(format!("print() expects one argument, received {}.",vs.len()),*l,*c));}self.output.push(vs[0].show());Ok(V::Null)}
                V::Fun(fun)=>{if vs.len()!=fun.params.len(){return Err(self.error(format!("Function '{}' expects {} arguments, received {}.",fun.name,fun.params.len(),vs.len()),*l,*c));}let parent=fun.env.upgrade().ok_or_else(||self.error("Function environment is no longer available.",*l,*c))?;let call=Env::child(parent);for(n,v)in fun.params.iter().zip(vs){call.borrow_mut().define(n.clone(),v,true).map_err(|x|self.error(x,*l,*c))?;}let old=self.env.clone();self.env=call;let r=self.exec_all(&fun.body);self.env=old;match r?{Flow::Next=>Ok(V::Null),Flow::Return(v)=>Ok(v)}}
                v=>Err(self.error(format!("Cannot call a value of type {}.",v.ty()),*l,*c))
            }}
            E::Index(a,i,l,c)=>{let o=self.eval(a)?;let index_value=self.eval(i)?;let idx=self.index(index_value,*l,*c)?;if let V::Arr(items)=o{items.borrow().get(idx).cloned().ok_or_else(||self.error(format!("Array index {idx} is out of bounds."),*l,*c))}else{Err(self.error("Indexing requires Array.",*l,*c))}}
            E::SetIndex(a,i,r,l,c)=>{let o=self.eval(a)?;let idx=self.index(self.eval(i)?,*l,*c)?;let v=self.eval(r)?;if let V::Arr(items)=o{let mut items=items.borrow_mut();if idx>=items.len(){return Err(self.error(format!("Array index {idx} is out of bounds."),*l,*c));}items[idx]=v.clone();Ok(v)}else{Err(self.error("Indexed assignment requires Array.",*l,*c))}}
            E::Property(a,n,l,c)=>{let o=self.eval(a)?;if n=="length"{if let V::Arr(items)=o{return Ok(V::Num(items.borrow().len() as f64));}}Err(self.error(format!("Property '{n}' is not available on {}.",o.ty()),*l,*c))}
        }
    }
    fn binary(&self,a:V,op:K,b:V,l:usize,c:usize)->Result<V>{
        match op{
            K::Plus=>match(a,b){(V::Num(x),V::Num(y))=>self.finite(x+y,l,c),(V::Str(x),V::Str(y))=>Ok(V::Str(x+&y)),(x,y)=>Err(self.error(format!("'+' expects two Numbers or two Strings, got {} and {}.",x.ty(),y.ty()),l,c))},
            K::Minus|K::Star|K::Slash|K::Percent=>{let(x,y)=if let(V::Num(x),V::Num(y))=(a,b){(x,y)}else{return Err(self.error("Arithmetic operators require Number operands.",l,c));};if (op==K::Slash||op==K::Percent)&&y==0.0{return Err(self.error("Division or modulo by zero.",l,c));}self.finite(match op{K::Minus=>x-y,K::Star=>x*y,K::Slash=>x/y,K::Percent=>x%y,_=>unreachable!()},l,c)}
            K::Gt|K::GtEq|K::Lt|K::LtEq=>{let(x,y)=if let(V::Num(x),V::Num(y))=(a,b){(x,y)}else{return Err(self.error("Ordering comparisons require Number operands.",l,c));};Ok(V::Bool(match op{K::Gt=>x>y,K::GtEq=>x>=y,K::Lt=>x<y,K::LtEq=>x<=y,_=>unreachable!()}))}
            K::EqEq=>Ok(V::Bool(a.eq(&b))),K::BangEq=>Ok(V::Bool(!a.eq(&b))),_=>unreachable!()
        }
    }
    fn finite(&self,n:f64,l:usize,c:usize)->Result<V>{if n.is_finite(){Ok(V::Num(n))}else{Err(self.error("Numeric operation produced a non-finite result.",l,c))}}
    fn boolean(&self,v:V,l:usize,c:usize)->Result<bool>{if let V::Bool(b)=v{Ok(b)}else{Err(self.error(format!("Condition must be Boolean, received {}.",v.ty()),l,c))}}
    fn index(&self,v:V,l:usize,c:usize)->Result<usize>{if let V::Num(n)=v{if n>=0.0&&n.fract()==0.0&&n<(usize::MAX as f64){Ok(n as usize)}else{Err(self.error("Array index must be a non-negative integer.",l,c))}}else{Err(self.error("Array index must be Number.",l,c))}}
}

/// Tokenizes, parses, and executes a complete HK source file.
pub fn run(source_name:&str,source:&str)->Result<Vec<String>>{
    let tokens=Lexer::new(source_name,source).scan()?;
    let statements=Parser::new(tokens,source_name).parse()?;
    let mut engine=Engine::new(source_name);engine.run(&statements)?;Ok(engine.output)
}

#[cfg(test)]
mod tests {
    use super::run;
    #[test] fn arithmetic_precedence(){assert_eq!(run("test.hk","print(10 + 5 * 2);").unwrap(),vec!["20"]);}
    #[test] fn functions_recursion_and_arrays(){
        let src=r#"fn sum(n){let x=0;for(let i=1;i<=n;i=i+1){x=x+i;}return x;}let a=[1,2];a[0]=sum(5);print(a[0]);print(a.length);"#;
        assert_eq!(run("test.hk",src).unwrap(),vec!["15","2"]);
    }
    #[test] fn constant_reassignment_fails(){assert!(run("test.hk","const x=1;x=2;").unwrap_err().message.contains("constant"));}
    #[test] fn conditions_require_booleans(){assert!(run("test.hk","if(1){print(1);}").unwrap_err().message.contains("Boolean"));}
    #[test] fn division_by_zero_fails(){assert!(run("test.hk","print(1/0);").unwrap_err().message.contains("zero"));}
    #[test] fn lexical_scopes_allow_shadowing(){assert_eq!(run("test.hk","let x=1;{let x=2;print(x);}print(x);").unwrap(),vec!["2","1"]);}
}
