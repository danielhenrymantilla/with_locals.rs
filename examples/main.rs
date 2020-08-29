use ::with_locals::with;

#[with]
fn hex (x: u32) -> &'self dyn ::core::fmt::Display
{
    &format_args!("{:#x}", x)
}

trait ToString {
    #[with]
    fn to_string (self: &'_ Self) -> &'self str
    {
        unimplemented!();
    }
}

impl ToString for u32 {
    #[with]
    fn to_string (self: &'_ u32) -> &'self str
    {
        let mut x = *self;
        if x == 0 {
            return "0";
        }
        let mut arr = [b'0'; 1 + 3 + 3 + 3]; // u32::MAX ~ 4_000_000_000
        let mut buf = &mut arr[..];
        while x > 0 {
            let (last, buf_) = buf.split_last_mut().unwrap();
            buf = buf_;
            *last = b'0' + (x % 10) as u8;
            x /= 10;
        }
        let len = buf.len();
        return ::core::str::from_utf8(&arr[len ..]).unwrap();
    }
}

#[with]
fn main ()
{
    ();
    {
        #[with]
        let s_hex = hex(66);
        #[with]
        let s_hex = hex(66);
        drop(s_hex);
        ();
        #[with]
        let n = ::core::u32::MAX.to_string();
        dbg!(n);
        println!("s_hex = `{}`", s_hex);
        assert_eq!(s_hex.to_string(), "0x42");
    }
}
