use std::fmt;
#[derive(Debug,Clone)]
pub struct HkError{pub code:&'static str,pub message:String,pub source:String,pub line:usize,pub column:usize,source_line:Option<String>}
impl HkError{pub(crate) fn new(code:&'static str,message:impl Into<String>,source:&str,line:usize,column:usize,source_text:&str)->Self{let source_line=source_text.lines().nth(line.saturating_sub(1)).map(str::to_owned);Self{code,message:message.into(),source:source.into(),line,column,source_line}}}
impl fmt::Display for HkError{fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{write!(f,"Error[{}]: {}\\n  --> {}:{}:{}",self.code,self.message,self.source,self.line,self.column)?;if let Some(line)=&self.source_line{write!(f,"\\n  |\\n{:>3} | {}\\n  | ",self.line,line)?;for ch in line.chars().take(self.column.saturating_sub(1)){if ch=='\\t'{write!(f,"\\t")?;}else{write!(f," ")?;}}write!(f,"^")?;}Ok(())}}
impl std::error::Error for HkError{}
