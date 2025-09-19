use std::any::type_name_of_val;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::mem;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

#[repr(C, align(8))]
union PrimitiveData<T: Copy + Hash + Eq>{
    val: T,
    rp: Option<NonNull<Vec<PrimitiveData<T>>>>,
}

pub struct PrimitiveMuted<T: Hash + Eq + Debug + Copy + Debug + Display> {
    data: Box<Vec<PrimitiveData<T>>>,
    r_hold: HashMap<NonNull<Vec<PrimitiveData<T>>>, Option<(ManuallyDrop<Box<Vec<PrimitiveData<T>>>>, usize, usize)>>,
    index_offset: isize,
    prefix_vec: (Vec<usize>, usize),
    t_is_32: bool,
    rc: usize,
}

impl<T: Hash + Eq + Debug + Copy + Debug + Display> Display for PrimitiveMuted<T>{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let len = *(self.prefix_vec.0.last()).unwrap();
        let mut display_raw: Vec<&T> = Vec::with_capacity(len);
        unsafe {
            for i in 0..len{
                let x = self.get_raw(i).unwrap();
                display_raw.push(x.0);
            }
            write!(f, "{:?}", display_raw)
        }
    }
}

impl<T: Hash + Eq + Debug + Copy + std::marker::Copy + ToString + std::fmt::Display> PrimitiveMuted<T> {
    pub fn new(other: Vec<T>, is_32bit: bool) -> PrimitiveMuted<T> {
        let (transmuted, index, len) = unsafe {PrimitiveMuted::transmute_vec(other, is_32bit)};

        return PrimitiveMuted{
            data: Box::new(transmuted),
            r_hold: HashMap::new(),
            index_offset: index as isize,
            prefix_vec: ((1..len + 1).collect(), len),
            t_is_32: is_32bit,
            rc: 0
        }
    }
    unsafe fn transmute_vec(vec: Vec<T>, convert_32: bool) -> (Vec<PrimitiveData<T>>, usize, usize) {

        let size_of_t = 8;
        let len = vec.len();

        let type_of = type_name_of_val(&vec[0]);

        if !convert_32 && !(type_of != "u64" || type_of != "i64"){
            panic!("liar liar pants on fire1 {}", type_of);
        }
        if convert_32 && (type_of == "u64" || type_of == "i64"){
            panic!("liar liar pants on fire2 {}", type_of);
        }

        if convert_32 {
            if type_of == "i32" {
                let mut trans: Vec<i64> = Vec::with_capacity(vec.len());
                let first_pass = mem::transmute::<Vec<T>, Vec<i32>>(vec);
                for v in first_pass.iter() {
                    trans.push(*v as i64);
                }
                return (mem::transmute::<Vec<i64>, Vec<PrimitiveData<T>>>(trans), size_of_t, len)
            }
            if type_of == "u32" {
                let mut trans: Vec<u64> = Vec::with_capacity(vec.len());
                let first_pass = mem::transmute::<Vec<T>, Vec<u32>>(vec);
                for v in first_pass.iter() {
                    trans.push(*v as u64);
                }
                return (mem::transmute::<Vec<u64>, Vec<PrimitiveData<T>>>(trans), size_of_t, len)
            }
        }

        if type_of == "i64"{
            return (mem::transmute::<Vec<T>, Vec<PrimitiveData<T>>>(vec), size_of_t, len)
        }
        if type_of == "u64" {
            return (mem::transmute::<Vec<T>, Vec<PrimitiveData<T>>>(vec), size_of_t, len)

        }else {
            panic!("Unsupported type: {}", type_of);
        }

    }

    pub fn push_vec(&mut self, other: Vec<T>) {
        let (trans_owo, _, len) = unsafe {PrimitiveMuted::transmute_vec(other, self.t_is_32)};
        let mut other = ManuallyDrop::new(Box::new(trans_owo));
        let raw: *mut Vec<PrimitiveData<T>> = &mut **other;
        let ptr = NonNull::new(raw).unwrap();
        let maybe_ptr = Some(ptr);
        self.r_hold.insert(ptr , Some((other, self.data.len(), len)));
        self.data.push(PrimitiveData{rp: maybe_ptr});
        let prefix_len = self.prefix_vec.0.len();
        let previous = &self.prefix_vec.0[prefix_len - 1];
        self.prefix_vec.0.push(len + previous);
        self.rc += 1;
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

    pub unsafe fn get_raw_mut(&mut self, index: usize) -> Option<&mut T> {
        let target = index + 1;
        let len = self.data.len();
        let rough_index = self.prefix_vec.0.partition_point(|&x| x < target);
        if rough_index >= len{
            panic!("index out of bounds");
        }

        let data = &mut self.data[rough_index];
        let ptr = match unsafe{(*data).rp}{
            Some(ptr) => ptr,
            None => panic!("Ts is none")
        };

        match self.r_hold.get_mut(&ptr) {
            Some(v) => {
                return match v {
                    Some(p) => {
                        let vec: &mut Vec<PrimitiveData<T>> = &mut *p.0;
                        let offset = if rough_index == 0{
                            index
                        }else {
                            index - self.prefix_vec.0[rough_index - 1]
                        };
                        match vec.get_mut(offset) {
                            Some(PrimitiveData {val: v}) => Some(v),
                            None => None
                        }
                    },
                    None => None
                }
            },
            None => {
                return Some(&mut (*data).val)
            }
        }


    }

    unsafe fn get_raw(&self, index: usize) -> Option<(&T, u8)> {
        let target = index + 1;
        let len = self.data.len();
        let rough_index = self.prefix_vec.0.partition_point(|&x| x < target);
        if rough_index >= len{
            panic!("index out of bounds");
        }

        let data = &self.data[rough_index];
        let ptr = match unsafe{(*data).rp}{
            Some(ptr) => ptr,
            None => panic!("Ts is none")
        };

        match self.r_hold.get_key_value(&ptr) {
            Some((_,v)) => {
                return match v {
                    Some(p) => {
                        let vec: &Vec<PrimitiveData<T>> = &**p.0;
                        let offset = if rough_index == 0{
                            index
                        }else {
                            index - self.prefix_vec.0[rough_index - 1]
                        };
                        match vec.get(offset) {
                            Some(PrimitiveData{val: v}) => Some((&v, 0)),
                            None => None
                        }
                    },
                    None => None
                }
            },
            None => match self.data.get(rough_index) {
                Some(PrimitiveData{val: v}) => Some((&v, 1)),
                None => None
            }
        }
    }

}