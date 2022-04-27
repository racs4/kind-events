use std::str;
use std::{
    fmt::{Error, Write as FmtWrite},
    num::ParseIntError,
    str::Utf8Error,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const WATCH: u8 = 0;
pub const UNWATCH: u8 = 1;
pub const POST: u8 = 2;
pub const SHOW: u8 = 3;
pub const TIME: u8 = 4;

type Hex = String;
type Bytes = Vec<u8>;

fn hex_to_bytes(hex: &str) -> Result<Bytes, ParseIntError> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect()
}

pub fn bytes_to_hex(buf: &[u8]) -> Hex {
    let mut s = String::with_capacity(buf.len() * 2);
    for &b in buf {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

fn hex_join(arr: &[Hex]) -> Hex {
    arr.join("")
}

pub fn hexs_to_bytes(arr: &[Hex]) -> Result<Bytes, ParseIntError> {
    hex_to_bytes(&hex_join(arr))
}

pub fn u8_to_hex(num: u8) -> Hex {
    num_to_hex(num.into())
}

pub fn u32_to_hex(num: u32) -> Hex {
    num_to_hex(num.into())
}

pub fn u64_to_hex(num: u64) -> Hex {
    num_to_hex(num)
}

fn num_to_hex(num: u64) -> Hex {
    let res = format!("{:02x}", num);
    if res.len() % 2 == 0 {
        res
    } else {
        format!("0{}", res)
    }
}

fn hex_to_u8(num: &Hex) -> Result<u8, ParseIntError> {
    u8::from_str_radix(num, 16)
}

pub fn hex_to_u32(num: &str) -> Result<u32, ParseIntError> {
    u32::from_str_radix(num, 16)
}

pub fn hex_to_u64(num: &Hex) -> Result<u64, ParseIntError> {
    u64::from_str_radix(num, 16)
}

pub fn check_hex(bits: usize, hex: &str) -> Result<Hex, String> {
    // if (!/^[a-fA-F0-9]*$/.test(hex)) {
    //   return null;
    // }
    if let Some(x) = hex.find(|c: char| !c.is_ascii_hexdigit()) {
        println!("Invalid character at {} in {}", x, hex);
        return Err(format!("Invalid character at {}", x));
    }

    let mut hex_aux = hex.to_string();
    if bits > 0 {
        while hex_aux.len() * 4 < bits {
            hex_aux = format!("0{}", hex_aux);
        }
        if hex_aux.len() * 4 > bits {
            hex_aux = hex_aux[0..(bits / 4)].to_string();
        }
        Ok(hex_aux.to_lowercase())
    } else {
        if hex.len() % 2 == 1 {
            hex_aux = format!("0{}", hex);
        }
        Ok(hex_aux.to_lowercase())
    }
}

pub fn string_to_bytes(str: &str) -> Bytes {
    str.as_bytes().to_vec()
}

fn bytes_to_string(buf: &[u8]) -> Result<&str, String> {
    str::from_utf8(buf).map_err(|x| x.to_string())
}

pub fn string_to_hex(str: &str) -> Hex {
    bytes_to_hex(&string_to_bytes(str))
}

fn hex_to_string<'a>(hex: &str) -> String {
    let bytes = hex_to_bytes(hex).unwrap();
    bytes_to_string(&bytes).unwrap().to_owned()
}

// Returns current time
pub fn get_time() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_millis() as u64
}

// Returns current tick
pub fn get_tick() -> u64 {
    (get_time() as f64 / 62.5).floor() as u64
}

//execute function in a time interval
// pub fn set_interval(func: &dyn(Fn() -> ()), dur: Duration) {
//   thread::spawn(|| {
//     loop {
//       func();
//       thread::sleep(dur);
//     }
//   });
// }

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_string_to_hex() {
        assert_eq!(string_to_hex(""), "");
        assert_eq!(string_to_hex("11111111"), "3131313131313131");
        assert_eq!(
            string_to_hex("cnsjdcnsdkcjnsdckjsndckjsdckjsdcn"),
            "636e736a64636e73646b636a6e7364636b6a736e64636b6a7364636b6a7364636e"
        );
        assert_eq!(
          string_to_hex("vasco da gama a sua fama assim se fez, tua imensa torcida é bem feliz!"),
          "766173636f2064612067616d612061207375612066616d6120617373696d2073652066657a2c2074756120696d656e736120746f726369646120c3a92062656d2066656c697a21"
        );
    }

    #[test]
    fn test_hex_to_string() {
        assert_eq!(hex_to_string(""), "");
        assert_eq!(hex_to_string("3131313131313131"), "11111111");
        assert_eq!(
            hex_to_string("636e736a64636e73646b636a6e7364636b6a736e64636b6a7364636b6a7364636e"),
            "cnsjdcnsdkcjnsdckjsndckjsdckjsdcn"
        );
        assert_eq!(
        hex_to_string("766173636f2064612067616d612061207375612066616d6120617373696d2073652066657a2c2074756120696d656e736120746f726369646120c3a92062656d2066656c697a21"),
        "vasco da gama a sua fama assim se fez, tua imensa torcida é bem feliz!"
      );
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(bytes_to_hex(&hex_to_bytes("0101").unwrap()), "0101");
        assert_eq!(
            bytes_to_hex(&hex_to_bytes("1234567890").unwrap()),
            "1234567890"
        );
    }

    proptest! {
        #[test]
        fn encode_decode_hex_string(s in "\\PC*") {
          let r = hex_to_string(&string_to_hex(&s));
          assert_eq!(r, s);
        }
    }
}
