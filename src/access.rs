use std::fmt;

macro_rules! bit_concat {
    ($($vals:ident)|+) => {
        $(
            Self::$vals.bits()
        )|*
    }
}

bitflags! {
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AccessFlag: u64 {
        const UNSET                   = 0;
        const PUBLIC                  = 1 << 1;
        const PRIVATE                 = 1 << 2;
        const PROTECTED               = 1 << 3;
        const STATIC                  = 1 << 4;
        const FINAL                   = 1 << 5;
        const SYNCHRONIZED            = 1 << 6;
        const BRIDGE                  = 1 << 7;
        const VARARGS                 = 1 << 8;
        const NATIVE                  = 1 << 9;
        const ABSTRACT                = 1 << 10;
        const STRICTFP                = 1 << 11;
        const SYNTHETIC               = 1 << 12;
        const CONSTRUCTOR             = 1 << 13;
        const DECLARED_SYNCHRONIZED   = 1 << 14;
        const INTERFACE               = 1 << 15;
        const ENUM                    = 1 << 16;
        const ANNOTATION              = 1 << 17;
        const VOLATILE                = 1 << 18;
        const TRANSIENT               = 1 << 19;

        // Android visibility spec


        const WHITELIST               = 1 << 20;
        const GREYLIST                = 1 << 21;
        const BLACKLIST               = 1 << 22;
        const GREYLIST_MAX_O          = 1 << 23;
        const GREYLIST_MAX_P          = 1 << 24;
        const GREYLIST_MAX_Q          = 1 << 25;

        // Some of these don't exist, but futureproof I guess
        const GREYLIST_MAX_R          = 1 << 26;
        const GREYLIST_MAX_S          = 1 << 27;
        const GREYLIST_MAX_T          = 1 << 28;
        const GREYLIST_MAX_U          = 1 << 29;
        const GREYLIST_MAX_V          = 1 << 30;


        const ANDROID_RESTRICTIONS = bit_concat!(
            WHITELIST | GREYLIST | BLACKLIST | GREYLIST_MAX_O | GREYLIST_MAX_P |
            GREYLIST_MAX_Q | GREYLIST_MAX_R | GREYLIST_MAX_S | GREYLIST_MAX_T |
            GREYLIST_MAX_U | GREYLIST_MAX_V
        );

        const CORE_PLATFORM_API = 1 << 40;
        const TEST_API = 1 << 41;
    }
}

impl Default for AccessFlag {
    fn default() -> Self {
        Self::UNSET
    }
}

impl AccessFlag {
    pub fn maybe_parse(s: &str) -> Option<AccessFlag> {
        let parsed = Self::parse(s);
        if AccessFlag::UNSET == parsed {
            None
        } else {
            Some(parsed)
        }
    }
    // TODO This should be implemented the same way we parse directives and
    // instructions with generated code
    pub fn parse(s: &str) -> AccessFlag {
        match s {
            "public" => AccessFlag::PUBLIC,
            "private" => AccessFlag::PRIVATE,
            "protected" => AccessFlag::PROTECTED,
            "static" => AccessFlag::STATIC,
            "final" => AccessFlag::FINAL,
            "synchronized" => AccessFlag::SYNCHRONIZED,
            "bridge" => AccessFlag::BRIDGE,
            "varargs" => AccessFlag::VARARGS,
            "native" => AccessFlag::NATIVE,
            "abstract" => AccessFlag::ABSTRACT,
            "strictfp" => AccessFlag::STRICTFP,
            "synthetic" => AccessFlag::SYNTHETIC,
            "constructor" => AccessFlag::CONSTRUCTOR,
            "declared-synchronized" => AccessFlag::DECLARED_SYNCHRONIZED,
            "interface" => AccessFlag::INTERFACE,
            "enum" => AccessFlag::ENUM,
            "annotation" => AccessFlag::ANNOTATION,
            "volatile" => AccessFlag::VOLATILE,
            "transient" => AccessFlag::TRANSIENT,
            "greylist" => AccessFlag::GREYLIST,
            "greylist-max-o" => AccessFlag::GREYLIST_MAX_O,
            "greylist-max-p" => AccessFlag::GREYLIST_MAX_P,
            "greylist-max-q" => AccessFlag::GREYLIST_MAX_Q,
            "greylist-max-r" => AccessFlag::GREYLIST_MAX_R,
            "greylist-max-s" => AccessFlag::GREYLIST_MAX_S,
            "greylist-max-t" => AccessFlag::GREYLIST_MAX_T,
            "greylist-max-u" => AccessFlag::GREYLIST_MAX_U,
            "greylist-max-v" => AccessFlag::GREYLIST_MAX_V,
            "whitelist" => AccessFlag::WHITELIST,
            "blacklist" => AccessFlag::BLACKLIST,
            "core-platform-api" => AccessFlag::CORE_PLATFORM_API,
            "test-api" => AccessFlag::TEST_API,
            _ => AccessFlag::UNSET,
        }
    }
}

impl AccessFlag {
    #[inline]
    pub fn is_public(&self) -> bool {
        self.contains(AccessFlag::PUBLIC)
            || !(self.contains(AccessFlag::PROTECTED) || self.contains(AccessFlag::PRIVATE))
    }

    pub fn ensure_access(&mut self) {
        let acc = AccessFlag::PUBLIC | AccessFlag::PROTECTED | AccessFlag::PRIVATE;
        if self.intersects(acc) {
            return;
        }
        self.insert(AccessFlag::PUBLIC);
    }
}

impl fmt::Display for AccessFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags: Vec<&'static str> = Vec::new();
        if self.contains(AccessFlag::PUBLIC) {
            flags.push("public");
        } else if self.contains(AccessFlag::PROTECTED) {
            flags.push("protected");
        } else if self.contains(AccessFlag::PRIVATE) {
            flags.push("private");
        } else {
            flags.push("public");
        }
        if self.contains(AccessFlag::STATIC) {
            flags.push("static");
        }

        if self.contains(AccessFlag::CONSTRUCTOR) {
            flags.push("constructor");
        }

        if self.contains(AccessFlag::SYNCHRONIZED) {
            flags.push("synchronized");
        }

        if self.contains(AccessFlag::TRANSIENT) {
            flags.push("transient");
        } else if self.contains(AccessFlag::VOLATILE) {
            flags.push("volatile");
        }

        if self.contains(AccessFlag::FINAL) {
            flags.push("final");
        }

        if self.contains(AccessFlag::VARARGS) {
            flags.push("varargs");
        }

        if self.contains(AccessFlag::SYNTHETIC) {
            flags.push("synthetic");
        } else if self.contains(AccessFlag::NATIVE) {
            flags.push("native");
        }

        if self.contains(AccessFlag::STRICTFP) {
            flags.push("strictfp");
        }
        if self.intersects(AccessFlag::ANDROID_RESTRICTIONS) {
            flags.push(if self.contains(AccessFlag::WHITELIST) {
                "whitelist"
            } else if self.contains(AccessFlag::GREYLIST) {
                "greylist"
            } else if self.contains(AccessFlag::GREYLIST_MAX_O) {
                "greylist-max-o"
            } else if self.contains(AccessFlag::GREYLIST_MAX_P) {
                "greylist-max-p"
            } else if self.contains(AccessFlag::GREYLIST_MAX_Q) {
                "greylist-max-q"
            } else if self.contains(AccessFlag::GREYLIST_MAX_R) {
                "greylist-max-r"
            } else if self.contains(AccessFlag::GREYLIST_MAX_S) {
                "greylist-max-s"
            } else if self.contains(AccessFlag::GREYLIST_MAX_T) {
                "greylist-max-t"
            } else if self.contains(AccessFlag::GREYLIST_MAX_U) {
                "greylist-max-u"
            } else if self.contains(AccessFlag::GREYLIST_MAX_V) {
                "greylist-max-v"
            } else {
                "blacklist"
            });
        }
        if self.contains(AccessFlag::CORE_PLATFORM_API) {
            flags.push("core-platform-api");
        }
        if self.contains(AccessFlag::TEST_API) {
            flags.push("test-api");
        }
        write!(f, "{}", flags.join(" "))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn access_strings() {
        let access = AccessFlag::PUBLIC | AccessFlag::STATIC | AccessFlag::FINAL;
        assert_eq!(access.to_string(), String::from("public static final"));
        let access = AccessFlag::UNSET;
        assert_eq!(access.to_string(), String::from("public"));
    }

    #[test]
    fn access_public() {
        let access = AccessFlag::UNSET;
        assert_eq!(access.is_public(), true);
        let access = AccessFlag::PUBLIC;
        assert_eq!(access.is_public(), true);
        let access = AccessFlag::PRIVATE;
        assert_eq!(access.is_public(), false);
        let access = AccessFlag::PROTECTED;
        assert_eq!(access.is_public(), false);
    }
}
