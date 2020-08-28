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
        let &(mut x) = self;
        let mut ret = [b' '; 1 + 3 + 3 + 3]; // u32::MAX ~ 4_000_000_000
        let mut buf = &mut ret[..];
        while x > 10 {
            *buf.last_mut().unwrap() = b'0' + (x % 10) as u8;
            x /= 10;
            let len = buf.len();
            buf = &mut buf[.. len - 1];
        }
        *buf.last_mut().unwrap() = b'0' + x as u8;
        let len = buf.len();
        let buf = &ret[len - 1 ..];
        return ::core::str::from_utf8(buf).unwrap();
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
