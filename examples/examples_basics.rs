#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
enum SomethingElse{
    This,
    That
}
#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
struct ComplexStructure{
    part1: u32,
    part2: i32,
    part3: SomethingElse,
}

fn main() {
    //generic and primitive behave similarly except the primitive one is around 3 times faster with primitive Eq types so u32/i32 and u64/i64
    //therefore does not allow complex types like structs... for most things just use the generic one or just look in the benchmark branch for more details
    let mut super_dangerous_reference: *mut Muted::generic::Muted<i32>;

    {
        let mut muted = Muted::generic::Muted::new(vec![1, 2, 3, 4, 5]);
        super_dangerous_reference = &mut muted;
        muted.push_vec_convert(vec![1, 2, 3, 4, 5]);
        //use convert method if not already in the right data type
        let converted = Muted::generic::Muted::muted_from(vec![1, 2, 3, 4, 5]);
        muted.push_vec(converted);
        /* this now works and is a lot faster so if you have spare time just convert it first
        the non converted method is O(1) compared to O(n)
        */
        println!("{}", muted); // [1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5]
        //indexing is quite easy really...
        let _ = muted.read(5); // reads index 5, this is immutable
        muted.write(0, 67); //write 67 at index 0
        //read and write both return an option to check whether its a success or failure
        //why not result you might ask? because i dont wanna.
        //if you want something mutable use get_raw_mut
        let m = unsafe { muted.get_raw_mut(1) };
        unsafe {m.unwrap().replace(10)};
        println!("{}", muted); // [67, 10, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5]
        //for obvious reasons its an unsafe function... but at least its an option type, right?

        //now to the fun part...
        let t = muted.drop_vec(Some(5), None);
        println!("{:?}", t); //if its Some the vec was dropped if its None the vec wasnt dropped (probably wrong index)
        //with the debug print you can visually see the vector was indeed dropped by the glaring "None" in the data field
        println!("{:?}", muted);
        //Muted { data: [Val(67), Val(10), Val(3), Val(4), Val(5), Rp(None), Rp(Some(0x1b9cce9ce30))]
        //want a vector RIGHT THERE?? okay..
        muted.insert_vec(5, vec![420, 67]).unwrap();
        println!("{:?}", muted);
        //now something is certainly there where the None used to be:
        //Muted { data: [Val(67), Val(10), Val(3), Val(4), Val(5), Rp(Some(0x1ff337acc70)), Rp(Some(0x1ff337acc30))]

        println!("{}", muted);
        //and indeed its there [67, 10, 3, 4, 5, 420, 67, 1, 2, 3, 4, 5]
    }
    //well there is actually not much more to it really... the dropping happens automatically
    //as you can see with the convenient scope i placed there
    println!("{:?}", unsafe {&*super_dangerous_reference});
    //note this does still print the old data: "Muted { data: [], r_hold: {}, prefix_vec: ([], 0), rc: 0 }" somewhat
    //but its actually freed, the pointers are explicitly removed and its not a leak and the OS can reclaim the memory whenever it wants

    //also about non-primitive types... well no problem really
    let mut muted_complex = Muted::generic::Muted::new(vec![
        ComplexStructure{
            part1: 1,
            part2: -1,
            part3: SomethingElse::That
        },
        ComplexStructure{
            part1: 100,
            part2: -500,
            part3: SomethingElse::This
        }
    ]);

    println!("{}", muted_complex); // [ComplexStructure { part1: 1, part2: -1, part3: That },
    // ComplexStructure { part1: 100, part2: -500, part3: This }]
    println!("{:?}", muted_complex.read(1).unwrap().part2); // -500
    
}