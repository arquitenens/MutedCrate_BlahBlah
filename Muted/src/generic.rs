use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::mem::ManuallyDrop;
use std::ptr;
use std::ptr::NonNull;

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum Data<T: Eq + Hash> {
    Val(T),
    Rp(Option<NonNull<Vec<Data<T>>>>),
}
#[derive(Debug, Eq, PartialEq)]
pub struct Muted<T: Hash + Eq + Debug> {
    data: Box<Vec<Data<T>>>,
    r_hold: HashMap<Data<T>, Option<(ManuallyDrop<Box<Vec<Data<T>>>>, usize, usize)>>,
    prefix_vec: (Vec<usize>, usize),
    rc: usize,
}
impl<T: Eq + Hash + Debug + Clone> Iterator for Muted<T>{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.prefix_vec.1 >= self.data.len() {
            return None;
        }

        let idx = self.prefix_vec.1;
        self.prefix_vec.1 += 1;

        Self::read(self, idx).cloned()
    }
}

impl<T: Eq + Hash + Debug> Drop for Muted<T>{
    fn drop(&mut self) {
        let mut to_drop: Vec<(NonNull<Vec<Data<T>>>, usize)> = Vec::new();
        for (idx, item) in self.data.iter_mut().enumerate() {
            if let Data::Rp(opt_ptr) = item {
                if let Some(ptr) = opt_ptr.take() {

                    to_drop.push((ptr, idx));
                }
            }
        }
        for (ptr, _) in to_drop {
            self.drop_vec(None, Some(ptr));
        }
        self.r_hold.clear();
        self.data.clear();
    }
}

impl<T: Eq + Hash + Debug> Display for Muted<T>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "{:?}", self.data).expect("data not available");
        }
        let m_vec = Some(&self.data);
        if let Some(m_vec) = m_vec {
            let mut display_vec: Vec<&T> = Vec::new();
            for i in m_vec.iter(){
                match i {
                    Data::Val(v) => display_vec.push(v),
                    Data::Rp(p1) => if let Some(p) = p1{
                        unsafe {
                            let vec: &Vec<Data<T>> = p.as_ref();
                            for i in vec.iter(){
                                match i {
                                    Data::Val(v) => display_vec.push(v),
                                    _ => unreachable!("please no nested pointers (may be added later)"),
                                }

                            }
                        }
                    }
                }
            }
        write!(f, "{:?}", display_vec)
    }else { 
            panic!("data not available");
        }
    }
}

impl<T: Hash + Eq + Debug> Muted<T>{
    pub fn new(vec: Vec<T>) -> Self{
        let len = vec.len();
        return Muted{
            data: Box::new(Self::muted_from(vec)),
            r_hold: HashMap::new(),
            prefix_vec: ((1..=len).collect(), len),
            rc: 0
        };
    }
    
    pub fn muted_from(other: Vec<T>) -> Vec<Data<T>>{
        let len = other.len();
        let cap = other.capacity();
        let mut new: Vec<Data<T>> = Vec::with_capacity(cap);

        let dst = new.as_mut_ptr();
        for (i, item) in other.into_iter().enumerate() {
            unsafe { ptr::write(dst.add(i), Data::Val(item)); }
        }
        unsafe { new.set_len(len); }
        return new;
    }
    pub fn push_vec_convert(&mut self, other: Vec<T>){
        let other = Self::muted_from(other);
        self.push_vec(other);
    }
    pub fn push_vec(&mut self, other: Vec<Data<T>>) {
        let len = other.len();
        let mut other = ManuallyDrop::new(Box::new(other));
        let ptr: &mut Vec<Data<T>> = &mut **other;
        let maybe_ptr = NonNull::new(ptr);
        self.r_hold.insert(Data::Rp(maybe_ptr), Some((other, self.data.len(), len)));
        self.data.push(Data::Rp(maybe_ptr));
        self.rc += 1;

        let last = self.prefix_vec.0.last().cloned().unwrap_or(0);
        self.prefix_vec.0.push(last + len);
        self.prefix_vec.1 = self.prefix_vec.0.len();
    }
    pub fn is_empty(&self) -> bool{
        self.data.is_empty()
    }
    pub fn len(&self) -> usize {
        *self.prefix_vec.0.last().unwrap()
    }

    pub fn drop_vec(&mut self, index: Option<usize>, maybe_ptr: Option<NonNull<Vec<Data<T>>>>) -> Option<()>{
        match (index, maybe_ptr) {
            (Some(_),Some(_)) => {
                panic!("choose either index or direct pointer");
            },
            (None, None) => {
                panic!("must provide index or pointer")
            }
            _ => (),
        }
        let mut real_index = None;
        if let Some(key_index) = self.r_hold.get_key_value(&Data::Rp(maybe_ptr)){
            if let Some(x) = key_index.1{
                real_index = Some(x.1);
            }
        }
        if let Some(u_index) = index{
            real_index = Some(u_index);
        }

        if let Some(held) = self.r_hold.remove(&Data::Rp(maybe_ptr)) {
            self.data[real_index.unwrap()] = Data::Rp(None);
            let boxed: Box<Vec<Data<T>>> = ManuallyDrop::into_inner(match held {
                Some(x) => x.0,
                None => unreachable!(),
            });
            std::mem::drop(boxed);
            self.rc -= 1;
            return Some(());
        }
        if real_index.is_some() && !self.data.is_empty(){
            let ptr = match &self.data[real_index.unwrap()] {
                Data::Rp(p) => p,
                Data::Val(_v) => return None,
            };
            if let Some(held) = self.r_hold.remove(&Data::Rp(*ptr)) {
                let boxed: Box<Vec<Data<T>>> = ManuallyDrop::into_inner(match held {
                    Some(x) => x.0,
                    None => unreachable!(),
                });
                
                std::mem::drop(boxed);
                self.rc -= 1;
            }else {
                return None;
            }
            self.data[real_index.unwrap()] = Data::Rp(None);
            return Some(());
        }else {
            return None;
        }
    }

    pub unsafe fn insert_vec_unchecked(&mut self, index: usize, other: Vec<T>) -> Option<()>{
        self.insert_vec_inner(index, other, true)
    }

    pub fn insert_vec(&mut self, index: usize, other: Vec<T>) -> Option<()>{
        if index >= self.data.len() { return None; }
        self.insert_vec_inner(index, other, false)
    }

    fn insert_vec_inner(&mut self, index: usize, vec: Vec<T>, skip_calibration: bool) -> Option<()>{
        let len = vec.len();
        let wrapped: Vec<Data<T>> = vec.into_iter().map(Data::Val).collect();
        let mut other = ManuallyDrop::new(Box::new(wrapped));
        let ptr= NonNull::new(&mut **other);
        if self.data[index] == Data::Rp(None) {
            self.r_hold.insert(Data::Rp(ptr), Some((other, index, len)));
            self.data[index] = Data::Rp(ptr);
        }else {
            {
                ManuallyDrop::into_inner(other);
            }
            return None;
        }
        self.rc += 1;
        if !skip_calibration{
            unsafe {self.calibrate_index(len)}
        }
        return Some(());
    }
    pub fn read(&mut self, index: usize) -> Option<&T>{
        unsafe {
            if let Some(immutable) = self.get_raw_mut(index) {
                return Some(&*immutable)
            }else {
                return None;
            }
        }
    }
    pub fn write(&mut self, index: usize, val: T) -> Option<()>{
        unsafe {
            if let Some(mutable) = self.get_raw_mut(index) {
                *mutable = val;
                return Some(());
            }else {
                return None;
            }
        }
    }
    pub unsafe fn get_raw_mut(&mut self, index: usize) -> Option<&mut T>{
        let target = index + 1;
        let rough_index = self.prefix_vec.0.partition_point(|&x| x < target);
        if rough_index >= self.data.len(){
            panic!("index out of bounds");
        }
        return match &mut self.data[rough_index] {
            Data::Val(v) => Some(&mut *v),
            Data::Rp(p) => {
                if let Some(pointer) = p{
                    unsafe {
                        let vec: &mut Vec<Data<T>> = &mut *pointer.as_ptr();
                        let offset = if rough_index == 0{
                            index
                        }else {
                            index - self.prefix_vec.0[rough_index - 1]
                        };
                        let len = vec.len();
                        match vec.get_mut(offset) {
                            None => panic!("read or write failed, index is out of bounds, index is {}, len is: {}", offset, len),
                            Some(Data::Val(v)) => Some(v),
                            Some(_) => unreachable!(),
                        }
                    }
                }else {
                    None
                }
            }

        };
    }

    unsafe fn calibrate_index(&mut self, added_len: usize){
        let added_len = added_len as usize;
        let start = self.prefix_vec.1;
        for i in start..self.prefix_vec.0.len(){
            self.prefix_vec.0[i] += added_len;
        }   
        
    }
    
}