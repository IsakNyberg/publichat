macro_rules! rep { ($_:tt; $($r:tt)+) => {$($r)+}; } // repeat _ times

macro_rules! build_buf {
    (
        $name:ident;        // identifier (name) followed by a `;`
        $($len:expr),+      // 1 or more expressions (lengths) separated by commas
        $(;$($rest:item)+)? // optional: `;` followed by one or more tt's
    ) => {
        pub mod $name {
            use crate::constants::*;

            pub const SIZE: usize = $( $len + )* 0;
            pub type Buf = [u8; SIZE];
            pub const DEFAULT: Buf = [0; SIZE];
            // const LEN: usize = $( ig!{[1] $len} + )* 0;
            // const SIZES: [usize; Self::LEN] = [ $( $len, )* ];

            type NTuple<'a> = ($( rep!($len; &'a [u8]), )*);
            type NTupleMut<'a> = ($( rep!($len; &'a mut [u8]), )*);

            #[allow(unused_assignments)]
            #[allow(clippy::eval_order_dependence)]
            pub fn split(buf: &Buf) -> NTuple {
                let mut buf = buf.as_slice();
                ($({let (cur, new) = buf.split_at($len); buf = new; cur},)*)
            }

            #[allow(unused_assignments)]
            #[allow(clippy::eval_order_dependence)]
            pub fn split_mut(buf: &mut Buf) -> NTupleMut {
                let mut buf = buf.as_mut_slice();
                ($({let (cur, new) = buf.split_at_mut($len); buf = new; cur},)*)
            }

            $( $( $rest )+ )?
        }
    };
}

use crate::constants::PADDING_SIZE;
const fn pad_buf<const L: usize>(pad: [u8; PADDING_SIZE]) -> [u8; L] {
    // Create an empty buffer, put pad in the beginning
    // and END_PADDING at the end. This function is const!
    let mut buf = [0; L];
    let mut i = 0;
    while {
        // for-loops aren't const :/
        buf[i] = pad[i];
        buf[buf.len() - PADDING_SIZE + i] = pad::END_PADDING[i];
        i += 1;
        i < PADDING_SIZE
    } {}
    buf
}
// apply pad_buf
macro_rules! prepad {
    ($pad:expr) => {
        use super::pad;
        pub type PadBuf = [u8; SIZE + 2 * PADDING_SIZE];
        pub const PREPAD: PadBuf = super::pad_buf($pad);
        pub fn pad_split(buf: &PadBuf) -> NTuple {
            split(buf[PADDING_SIZE..][..SIZE].try_into().unwrap())
        }
        pub fn pad_split_mut(buf: &mut PadBuf) -> NTupleMut {
            split_mut((&mut buf[PADDING_SIZE..][..SIZE]).try_into().unwrap())
        }
    };
}

// ACTUAL DEFINITIONS:

// misc
// splitting not needed, make for consistency
build_buf!(hash; HASH_SIZE);
build_buf!(signature; SIGNATURE_SIZE);
build_buf!(qry_arg; QUERY_ARG_SIZE);
build_buf!(pad; PADDING_SIZE;
    // server -> client
    pub const SEND_PADDING:  [u8; PADDING_SIZE] = *b"snd";
    pub const FETCH_PADDING: [u8; PADDING_SIZE] = *b"fch";
    pub const QUERY_PADDING: [u8; PADDING_SIZE] = *b"qry";
    pub const END_PADDING:   [u8; PADDING_SIZE] = *b"end";

    // client -> server
    pub const MSG_PADDING:   [u8; PADDING_SIZE] = *b"msg";
);

// server -> client
build_buf!(msg_head; PADDING_SIZE, 1, MSG_ID_SIZE, 1;
    pub use super::pad::MSG_PADDING as PAD;  // includes padding
);
build_buf!(msg_out_c; TIME_SIZE, CYPHER_SIZE, SIGNATURE_SIZE);
build_buf!(msg_out_s; TIME_SIZE, CYPHER_SIZE + SIGNATURE_SIZE);
// same size as msg_out, but combines cypher with signature
// because server doesn't need the distinction.

// client -> server
build_buf!(fetch; CHAT_ID_SIZE; prepad!(pad::FETCH_PADDING););
build_buf!(query; CHAT_ID_SIZE, 1, MSG_ID_SIZE; prepad!(pad::QUERY_PADDING););
build_buf!(msg_in_c; CHAT_ID_SIZE, CYPHER_SIZE, SIGNATURE_SIZE; prepad!(pad::SEND_PADDING););
build_buf!(msg_in_s; CHAT_ID_SIZE, CYPHER_SIZE + SIGNATURE_SIZE);

// client-side
build_buf!(cypher; CYPHER_CHAT_KEY_SIZE, TIME_SIZE, HASH_SIZE, CYPHER_PAD_MSG_SIZE);
