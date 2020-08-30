use ::with_locals::with;
use ::core::fmt::Display;

#[with]
fn hex (n: u32) -> &'self dyn Display
{
    &format_args!("{:#x}", n)
}

impl ToStr for u32 {
    #[with('local)]
    fn to_str (self: &'_ u32) -> &'local str
    {
        let mut x = *self;
        if x == 0 { return "0"; }
        let mut buf = [b' '; 1 + 3 + 3 + 3]; // u32::MAX ~ 4_000_000_000
        let mut cursor = &mut buf[..];
        while x > 0 {
            let (last, cursor_) = cursor.split_last_mut().unwrap();
            cursor = cursor_;
            *last = b'0' + (x % 10) as u8;
            x /= 10;
        }
        let len = cursor.len();
        ::core::str::from_utf8(&buf[len ..])
            .unwrap()
    }
}

#[with]
fn main ()
{
    #[with]
    let s_hex = hex(66);
    println!("s_hex = {}", s_hex);
    assert_eq!(s_hex.to_string(), "0x42");

    if false {
        #[with]
        let s_hex: &dyn Display = hex(66);
        drop(s_hex);
    }

    #[with]
    let n: &str = ::core::u32::MAX.to_str();
    dbg!(n);
    assert_eq!(n.parse(), Ok(::core::u32::MAX));

    struct Roman(u8);
    impl ToStr for Roman {
        #[with]
        fn to_str (self: &'_ Self) -> &'self str
        {
            let mut buf = [b' '; 1 + 4 + 4];  // C LXXX VIII or CC XXX VIII
            let mut start = buf.len();
            let mut prepend = |b| {
                start = start.checked_sub(1).unwrap();
                buf[start] = b;
            };
            let mut n = self.0;
            if n == 0 {
                panic!("Vade retro!");
            }
            const DIGITS: [u8; 7] = *b"IVXLCDM";
            (0 ..= 2).for_each(|shift| {
                #[allow(nonstandard_style)]
                let (I, V, X) = (
                    DIGITS[2 * shift],
                    DIGITS[2 * shift + 1],
                    DIGITS[2 * shift + 2],
                );
                match n % 10 {
                    | units @ 0 ..= 3 => (0 .. units).for_each(|_| prepend(I)),
                    | 4 => {
                        prepend(V);
                        prepend(I);
                    },
                    | units @ 5 ..= 8 => {
                        (0 .. units - 5).for_each(|_| prepend(I));
                        prepend(V);
                    },
                    | 9 => {
                        prepend(X);
                        prepend(I);
                    },
                    | _ => unreachable!(),
                }
                n /= 10;
            });
            return ::core::str::from_utf8(&buf[start ..]).unwrap();
        }
    }

    for n in 1 ..= u8::MAX {
        #[with]
        let roman = Roman(n).to_str();
        println!("{:3} = {}", n, roman);
    }
}

trait ToStr {
    #[with('local)]
    fn to_str (self: &'_ Self) -> &'local str
    ;
}

/// User of of the trait
struct Displayable<T : ToStr>(T);
impl<T : ToStr> Display for Displayable<T> {
    #[with]
    fn fmt (self: &'_ Self, fmt: &'_ mut ::core::fmt::Formatter<'_>)
      -> ::core::fmt::Result
    {
        #[with]
        let s: &str = self.0.to_str();
        fmt.write_str(s)
    }
}
