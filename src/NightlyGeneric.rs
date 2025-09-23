
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::mem::ManuallyDrop;
use std::ptr;
use std::ptr::NonNull;
use crate::raw_buf::{offset, PrivateTypes, RawBuf, BIT};
use crate::raw_buf::offset::Bit;

//#[derive(Debug, Eq, Hash, PartialEq)]

#[repr(C, align(8))]
pub union Data<T: Eq + Hash> {
    val: ManuallyDrop<T>,
    rp: Option<NonNull<Vec<Data<T>>>>,
}

const VALUE: u8 = 2;
const RP: u8 = 1;



//#[derive(Debug, Eq, PartialEq)]
pub struct Muted<T: Hash + Eq + Debug> {
    data: Box<Vec<Data<T>>>,
    r_hold: HashMap<usize, Option<(ManuallyDrop<Box<Vec<Data<T>>>>, usize, usize)>>,
    prefix_vec: (Vec<usize>, usize),
    pub variant_marker: RawBuf,
    rc: usize,
}

#[macro_export]
macro_rules! muted_nightly {
    () => {
        crate::generic::Muted::new(vec![])
    };
    ($($element:expr),+) => {{
        let mut x = Vec::new();
        $(x.push(Data::Val($element));)+
        let mut y = $crate::generic::Muted::new_no_conv(x);
        y
    }};
    ($element:expr; $count:expr) => {{
        $crate::Muted::new(vec![$element; $count])
    }}
}


/*
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

 */

impl<T: Eq + Hash + Debug> Drop for Muted<T>{
    fn drop(&mut self) {
        let mut to_drop: Vec<(NonNull<Vec<Data<T>>>, usize)> = Vec::new();
        for (idx, item) in self.data.iter_mut().enumerate() {
            if self.variant_marker.read_bit(idx as u32) == RP {
                unsafe {
                    let ptr = item as *mut Data<T> as *mut Vec<Data<T>>;
                    to_drop.push((NonNull::new_unchecked(ptr), idx));
                }
            }
        }
        for (ptr, _) in to_drop {
            self.drop_vec(None, Some(ptr));
        }
        self.r_hold.clear();
        self.data.clear();
        self.variant_marker.data.clear();
        self.prefix_vec.0.clear();
        self.prefix_vec.1 = 0;
    }
}

impl<T: Eq + Hash + Debug> Display for Muted<T>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        /*
        if f.alternate() {
            write!(f, "{:?}", self.data).expect("data not available");
        }
         */
        let m_vec = Some(&self.data);
        if let Some(m_vec) = m_vec {
            let mut display_vec: Vec<&T> = Vec::new();
            for (i,v) in m_vec.iter().enumerate(){
                let start = (i as u32) * 2;
                let end   = start + 2;
                let variant = match self.variant_marker.read_bits(start, end, false, 0u8).unwrap() {
                    PrivateTypes::U8(variant) => variant,
                    _ => unreachable!()
                };
                if variant == VALUE{
                    let v = unsafe { &*v.val };
                    display_vec.push(v);
                }
                if variant == RP{
                        unsafe {
                            let p = v.rp.as_ref().expect("rp is none");
                            let vec: &Vec<Data<T>> = p.as_ref();
                            for (_, v) in vec.iter().enumerate(){
                                let start = (i as u32 * 2) + 2;
                                let end   = start + 2;
                                let variant = match self.variant_marker.read_bits(start, end, false, 0u8).unwrap() {
                                    PrivateTypes::U8(variant) => variant,
                                    _ => unreachable!()
                                };
                                let is_val = variant == VALUE;
                                if is_val {
                                    let v = unsafe { &*v.val };
                                    display_vec.push(v);
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
        let mut buf = RawBuf::new((len as u32 / 8u32) * 2u32 + 2);
        for _ in 0..len{
            buf.write_bits(offset::Bit(0), 2, 2, true);
        }

        return Muted{
            data: Box::new(Self::muted_from(vec)),
            r_hold: HashMap::new(),
            prefix_vec: ((1..=len).collect(), len),
            variant_marker: buf,
            rc: 0
        };
    }
    pub fn new_no_conv(vec: Vec<Data<T>>) -> Self{
        let len = vec.len();
        let mut buf = RawBuf::new((len as u32 / 8u32) * 2u32 + 2);
        buf.write_bits(offset::Bit(0), 2, 2, true);
        return Muted{
            data: Box::new(vec),
            r_hold: HashMap::new(),
            prefix_vec: ((1..=len).collect(), len),
            variant_marker: buf,
            rc: 0
        };
    }

    pub fn muted_from(other: Vec<T>) -> Vec<Data<T>>{
        let len = other.len();
        let cap = other.capacity();
        let mut new: Vec<Data<T>> = Vec::with_capacity(cap);

        let dst = new.as_mut_ptr();
        for (i, item) in other.into_iter().enumerate() {
            unsafe { ptr::write(dst.add(i), Data { val: ManuallyDrop::new(item) }) ; }
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
        let ptr_hash = ptr as *mut Vec<Data<T>> as usize;
        let maybe_ptr = NonNull::new(ptr);
        self.r_hold.insert(ptr_hash, Some((other, self.data.len(), len)));
        self.data.push(Data { rp: maybe_ptr });
        self.rc += 1;
        self.variant_marker.write_bits(Bit(0), 1, 2, true);
        for _ in 0..len{
            self.variant_marker.write_bits(Bit(0), 2, 2 as u32, true);
        }

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
        if let Some(key_index) = self.r_hold.get_key_value(&(maybe_ptr.unwrap().as_ptr() as *mut _ as usize)){
            if let Some(x) = key_index.1{
                real_index = Some(x.1);
            }
        }
        if let Some(u_index) = index{
            real_index = Some(u_index);
        }

        if let Some(held) = self.r_hold.remove(&(maybe_ptr.unwrap().as_ptr() as *mut _ as usize)) {
            self.data[real_index.unwrap()] = Data {rp: None};
            let boxed: Box<Vec<Data<T>>> = ManuallyDrop::into_inner(match held {
                Some(x) => x.0,
                None => unreachable!(),
            });
            std::mem::drop(boxed);
            self.rc -= 1;
            return Some(());
        }
        if real_index.is_some() && !self.data.is_empty(){
            let mut ptr= if self.variant_marker.read_bit(real_index.unwrap() as u32) == RP {
                &unsafe { self.data[real_index.unwrap()].rp }
            }else { 
                return None
            };
            
            if let Some(held) = self.r_hold.remove(&(ptr.unwrap().as_ptr() as *mut _ as usize)) {
                let boxed: Box<Vec<Data<T>>> = ManuallyDrop::into_inner(match held {
                    Some(x) => x.0,
                    None => unreachable!(),
                });

                std::mem::drop(boxed);
                self.rc -= 1;
            }else {
                return None;
            }
            self.data[real_index.unwrap()] = Data {rp: None};
            return Some(());
        }else {
            return None;
        }
    }
    
    /*
    pub unsafe fn insert_vec_unchecked(&mut self, index: usize, other: Vec<T>) -> Option<()>{
        self.insert_vec_inner(index, other, true)
    }

    pub fn insert_vec(&mut self, index: usize, other: Vec<T>) -> Option<()>{
        if index >= self.data.len() { return None; }
        self.insert_vec_inner(index, other, false)
    }
    
    
    fn insert_vec_inner(&mut self, index: usize, vec: Vec<T>, skip_calibration: bool) -> Option<()>{
        let len = vec.len();
        let wrapped: Vec<Data<T>> = vec.into_iter().map(Data{_: val}).collect();
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
    
     */
    pub fn read(&self, index: usize) -> Option<&T>{
        unsafe {
            if let Some(immutable) = self.get_raw_mut(index) {
                return Some(immutable.as_ref())
            }else {
                return None;
            }
        }
    }
    pub fn write(&mut self, index: usize, val: T) -> Option<()>{
        unsafe {
            if let Some(mutable) = self.get_raw_mut(index) {
                mutable.replace(val);

                return Some(());
            }else {
                return None;
            }
        }
    }
    pub unsafe fn get_raw_mut(&self, index: usize) -> Option<NonNull<T>>{
        let target = index + 1;
        let rough_index = self.prefix_vec.0.partition_point(|&x| x < target);
        if rough_index >= self.data.len(){
            panic!("index out of bounds");
        }
        return if self.variant_marker.read_bit(rough_index as u32) == RP {
            let v = &self.data[rough_index];
            Some(NonNull::from(unsafe { &*v.val }))
        } else {
                let p = &self.data[rough_index];
                let p = unsafe {p.rp.as_ref()};
                if let Some(pointer) = p{
                    unsafe {
                        let vec: &mut Vec<Data<T>> = &mut *pointer.as_ptr();
                        let offset = if rough_index == 0{
                            index
                        }else {
                            index - self.prefix_vec.0[rough_index - 1]
                        };
                        let len = vec.len();
                        return match vec.get_mut(offset) {
                            None => panic!("read or write failed, index is out of bounds, index is {}, len is: {}", offset, len),
                            Some(x) => if self.variant_marker.read_bit(rough_index as u32) == RP {
                                unreachable!("No nested pointers")
                            }else {
                                let v = unsafe {&mut x.val as *mut ManuallyDrop<T>};
                                Some(NonNull::new_unchecked(v.cast()))
                            }
                        }
                    }
                }else {
                    None
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
