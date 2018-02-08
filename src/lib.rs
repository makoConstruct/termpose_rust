#![feature(nll)]
#![feature(unreachable)]


//a full explaination of where this is at:
//I can't implement PartialEq
//I can't implement PartialEq because it needs a method with self having an anonymous lifetime. In order to evaluate the eq function I need to call a method on self that needs the named lifetime that is a part of the Self type, it isn't sure it can prove that the anonymous lifetime will last long enough to suffice the lifetime. nll borrowck gives a crap error for this, the old borrowck is clearer.
//I can't have the lifetime of the guard type and the associated types be anonymous, that is simply not allowed.
//The associated Iter type can't be parametric, because that feature is unstable, upon being unstable, the feature doesn't work (Self::Iter<'a> will be a compile error. Same goes for actual type params, it seems to be unable to talk about generic associated types at all,)
//I tried getting rid of the associated type, but there are still problems with traits and named lifetimes; it becomes impossible to abstract over St as well, because you can't deref to a str without naming a lifetime. All of this might be possible with generic associated types.

//I am very unhappy.



// extern crate string_cache;
// use string_cache::DefaultAtom as Strin;
// use std::ops::Deref;
use std::mem::unreachable;
use std::cmp::{PartialEq};
use std::result::Result;
// use std::debug_assert;
extern crate ref_slice;
use ref_slice::ref_slice;


// pub trait Term where Self:Sized {
// 	fn initial_string(& self)-> &str;
// 	fn contents<'a>(&'a self)-> std::slice::Iter<'a, Self>;
// 	fn tail<'a>(&'a self)-> std::slice::Iter<'a, Self>;
// }



// impl<'a, St> PartialEq for &'a Terms<St> {
// 	fn eq(&self, other: &&Terms<St>)-> bool {
// 		self == other
// 	}
// }

impl<'a, 'b> PartialEq for Terms<'a> {
	fn eq(&self, other: &Self)-> bool {
		
		match *self {
			Atom(_)=> true,
			_=> false
		};
		match *self {
			Atom(ref sa)=> {
				match *other {
					Atom(ref so)=> sa == so,
					_=> false,
				}
			}
			List(ref la)=> {
				match *other {
					List(ref lo)=> la == lo,
					_=> false,
				}
			}
		}
		// let comp = {
		// 	let st = self.tail();
		// 	let ot = other.tail();
		// 	st.eq(ot)
		// };
		// self.initial_string() == other.initial_string() && comp
	}
}

pub enum Terms<'a> {
	List(Vec<Terms<'a>>),
	Atom(&'a str),
}
pub use Terms::*;


impl<'a> Into<Terms<'a>> for &'a str {
	fn into(self) -> Terms<'a> { Atom(self) }
}

fn tail<I:Iterator>(mut v:I)-> I {
	v.next();
	v
}

impl<'a> Terms<'a> {
	pub fn initial_string(& self)-> &'a str { //if it bottoms out at an empty list, it returns the empty str
		match *self {
			List(ref v)=> {
				if let Some(ref ss) = v.first() {
					ss.initial_string()
				}else{
					""
				}
			}
			Atom(ref v)=> v,
		}
	}
	pub fn to_string(&self)-> String {
		let mut ret = String::new();
		self.stringify(&mut ret);
		ret
	}
	pub fn stringify(&self, s:&mut String){
		match *self {
			List(ref v)=> {
				s.push('(');
				for su in v.iter() {
					su.stringify(s);
					s.push(' ');
				}
				s.push(')');
			}
			Atom(ref v)=> {
				s.push_str(v);
			}
		}
	}
	pub fn contents<'b>(&'b self)-> std::slice::Iter<'b, Self> where 'a:'b { //if Atom, returns a slice iter of a single element that is the atom's str
		match *self {
			List(ref v)=> v.iter(),
			Atom(_)=> ref_slice(self).iter(),
		}
	}
	pub fn tail<'b>(&'b self)-> std::slice::Iter<'b, Self> where 'a:'b { //if Atom, returns an empty slice iter
		match *self {
			List(ref v)=> tail(v.iter()),
			Atom(_)=> [].iter(),
		}
	}
}

#[macro_export]
macro_rules! list {
	($($inner:expr),*)=> {{
		let mut list = Vec::new();
		$(list.push($inner.into()));*;
		List(list.into())
	}};
}


// pub fn check_str<'a>(v:&Terms<'a>)-> CheckResult<&'a str> { //would it make sense to match "str", ("str"), (("str")), but not ("str" "str")? Would that be easier if I were using slices instead of slice iters? Wouldn't a lot of things?
// 	match v.contents() {
		
// 	}
// }

pub fn parse_sexp<'a>(s:&'a str)-> Result<Terms<'a>, PositionedError> {
	ParserState::begin(s).parse()
}




struct ParserState<'a>{
	root:Vec<Terms<'a>>,
	stack:Vec<*mut Vec<Terms<'a>>>,
	iter:std::str::Chars<'a>,
	started_eating: &'a str,
	eating_str_mode:bool,
	line:usize,
	column:usize,
}

unsafe fn seriously_pop<T>(v: &mut Vec<T>)-> T {
	seriously_unwrap(v.pop())
}

fn seriously_unreach(s:&str)-> ! {
	if cfg!(debug_assertions) { panic!("{}", s) }
	else{ unsafe{ unreachable() } }
}

unsafe fn serious_get_back_mut<T>(v:&mut Vec<T>)-> &mut T {
	if let Some(v) = v.last_mut() { v }
	else{ seriously_unreach("this vec should never be empty"); }
}

unsafe fn seriously_unwrap<T>(v:Option<T>)-> T {
	if let Some(va) = v { va }
	else{ seriously_unreach("this option should have been some"); }
}

#[derive(Debug)]
pub struct PositionedError{
	pub line:usize, pub column:usize, pub message:String,
}

// fn find_ptr<'a>(vl:&Vec<Terms<'a>>, p:*const Vec<Terms<'a>>)-> bool {
// 	if vl as *const _ == p {
// 		true
// 	}else{
// 		vl.iter().any(|v:&Terms<'a>|{
// 			if let List(ref lr) = *v {
// 				find_ptr(lr, p)
// 			}else{
// 				false
// 			}
// 		})
// 	}
// }

impl<'a> ParserState<'a>{
	fn begin(s:&'a str)-> ParserState {
		ParserState{
			root: Vec::new(),
			stack: Vec::new(),
			iter: s.chars(),
			started_eating: "",
			eating_str_mode: false,
			line: 0,
			column: 0,
		}
	}
	fn last_list<'b>(&'b mut self)-> &'b mut Vec<Terms<'a>> where 'a:'b {
		let ret = if self.stack.len() > 0 {
			unsafe{ *serious_get_back_mut(&mut self.stack) }
		}else{
			&mut self.root
		};
		unsafe{ &mut *ret }
	}
	fn pinch_off_string_if_eating<'b>(&'b mut self, ending:&'a str) where 'a:'b {
		if self.eating_str_mode {
			let len = ending.as_ptr() as usize - self.started_eating.as_ptr() as usize;
			let s = unsafe{ self.started_eating.slice_unchecked(0, len) };
			self.last_list().push(Atom(s));
			self.started_eating = "";
			self.eating_str_mode = false;
		}
	}
	unsafe fn close_scope<'b>(&'b mut self) where 'a:'b {
		seriously_pop(&mut self.stack);
	}
	fn new_scope<'b>(&'b mut self) where 'a:'b {
		self.last_list().push(List(Vec::new()));
		//potential unnecessary indexing in last_mut, `push` already knows the index. Could be mitigated with a place_back
		unsafe{
			if let List(ref mut final_list_ref) = *seriously_unwrap(self.last_list().last_mut()) {
				let flp = final_list_ref as *mut _;
				self.stack.push(flp);
			}else{
				seriously_unreach("this should never happen");
			}
		}
	}
	fn parse(mut self)-> Result<Terms<'a>, PositionedError> {
		loop {
			let str_starting_here = self.iter.as_str();
			if let Some(c) = self.iter.next() {
				match c {
					' ' | '\t' => {
						self.pinch_off_string_if_eating(str_starting_here);
						self.column += 1;
					}
					'\n' => {
						self.pinch_off_string_if_eating(str_starting_here);
						self.column = 0;
						self.line += 1;
					}
					'(' => {
						self.pinch_off_string_if_eating(str_starting_here);
						self.new_scope();
						self.column += 1;
					}
					')' => {
						if self.stack.len() == 0 {
							return Err(PositionedError{ line:self.line, column:self.column, message:"excess paren".into() });
						}else{
							self.pinch_off_string_if_eating(str_starting_here);
							debug_assert!(self.stack.len() > 0);
							unsafe{ self.close_scope(); }
						}
						self.column += 1;
					}
					_ => {
						if !self.eating_str_mode {
							self.eating_str_mode = true;
							self.started_eating = str_starting_here;
						}
						self.line += 1;
					}
				}
			}else{
				if self.stack.len() > 1 {
					return Err(PositionedError{ line:self.line, column:self.column, message:"paren left open before the end".into() });
				}
				self.pinch_off_string_if_eating(str_starting_here);
				break;
			}
		}
		
		Ok(List(self.root.into()))
	}
}






#[cfg(test)]
mod tests {
	use ::*;
	#[test]
	fn it_work() {
		let ti = list!("hue", list!("when", "you", "now"), "then", "else");
		let to = parse_sexp("hue (when you now) then else").unwrap();
		println!("the parsed is {}", to.to_string());
		assert!(ti == to);
	}
}
