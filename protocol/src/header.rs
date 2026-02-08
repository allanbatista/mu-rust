//! Packet header representations translated from the legacy C++ server.

#![allow(non_camel_case_types)]

/// "C1"/"C3" framed packet header (byte-sized length).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PBMSG_HEAD {
    pub r#type: u8,
    pub size: u8,
    pub head: u8,
}

/// "C1"/"C3" packet header with sub-command byte.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PSBMSG_HEAD {
    pub r#type: u8,
    pub size: u8,
    pub head: u8,
    pub subh: u8,
}

/// "C2"/"C4" framed packet header (word-sized length).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PWMSG_HEAD {
    pub r#type: u8,
    pub size: [u8; 2],
    pub head: u8,
}

/// "C2"/"C4" framed packet header with sub-command byte.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PSWMSG_HEAD {
    pub r#type: u8,
    pub size: [u8; 2],
    pub head: u8,
    pub subh: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbmsg_head_layout_matches_cpp() {
        assert_eq!(core::mem::size_of::<PBMSG_HEAD>(), 3);
    }

    #[test]
    fn psbmsg_head_layout_matches_cpp() {
        assert_eq!(core::mem::size_of::<PSBMSG_HEAD>(), 4);
    }

    #[test]
    fn pwmsg_head_layout_matches_cpp() {
        assert_eq!(core::mem::size_of::<PWMSG_HEAD>(), 4);
    }

    #[test]
    fn pswmsg_head_layout_matches_cpp() {
        assert_eq!(core::mem::size_of::<PSWMSG_HEAD>(), 5);
    }
}
