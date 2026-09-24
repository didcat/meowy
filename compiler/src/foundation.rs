use crate::hir::FoundationType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Module {
    Core,
    Debug,
    Memory,
    Strings,
    Proof,
    Bits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BitOp {
    And,
    Or,
    Xor,
    Not,
}

impl BitOp {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::And => "bits.and",
            Self::Or => "bits.or",
            Self::Xor => "bits.xor",
            Self::Not => "bits.not",
        }
    }

    pub(crate) fn operator(self) -> &'static str {
        match self {
            Self::And => "&",
            Self::Or => "|",
            Self::Xor => "^",
            Self::Not => "~",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Descriptor {
    Always,
    Never,
    Indeterminable,
    Result,
    Flags,
}

impl Descriptor {
    pub(crate) fn resolve(module: Module, name: &str) -> Option<Self> {
        if module != Module::Proof {
            return None;
        }
        Some(match name {
            "Always" => Self::Always,
            "Never" => Self::Never,
            "Indeterminable" => Self::Indeterminable,
            "Result" => Self::Result,
            "Flags" => Self::Flags,
            _ => return None,
        })
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Always => "proof.Always",
            Self::Never => "proof.Never",
            Self::Indeterminable => "proof.Indeterminable",
            Self::Result => "proof.Result",
            Self::Flags => "proof.Flags",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    Type(FoundationType),
    Heap,
    StringCopy,
    CanCopy,
    Bits(BitOp),
}

impl Module {
    pub(crate) fn resolve(name: &str) -> Option<Self> {
        match name {
            "core" => Some(Self::Core),
            "debug" => Some(Self::Debug),
            "memory" => Some(Self::Memory),
            "strings" => Some(Self::Strings),
            "proof" => Some(Self::Proof),
            "bits" => Some(Self::Bits),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Debug => "debug",
            Self::Memory => "memory",
            Self::Strings => "strings",
            Self::Proof => "proof",
            Self::Bits => "bits",
        }
    }

    pub(crate) fn partial(self) -> bool {
        matches!(
            self,
            Self::Memory | Self::Strings | Self::Proof | Self::Bits
        )
    }

    pub(crate) fn item(self, name: &str) -> Option<Item> {
        match (self, name) {
            (Self::Bits, "and") => Some(Item::Bits(BitOp::And)),
            (Self::Bits, "or") => Some(Item::Bits(BitOp::Or)),
            (Self::Bits, "xor") => Some(Item::Bits(BitOp::Xor)),
            (Self::Bits, "not") => Some(Item::Bits(BitOp::Not)),
            (Self::Proof, "can_copy") => Some(Item::CanCopy),
            (Self::Memory, "Allocator") => Some(Item::Type(FoundationType::Allocator)),
            (Self::Memory, "AllocationFailure") => {
                Some(Item::Type(FoundationType::AllocationFailure))
            }
            (Self::Memory, "heap") => Some(Item::Heap),
            (Self::Strings, "Owned") => Some(Item::Type(FoundationType::OwnedString)),
            (Self::Strings, "copy") => Some(Item::StringCopy),
            _ => None,
        }
    }
}

impl Item {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Type(ty) => ty.name(),
            Self::Heap => "memory.heap",
            Self::StringCopy => "strings.copy",
            Self::CanCopy => "proof.can_copy",
            Self::Bits(op) => op.name(),
        }
    }
}
