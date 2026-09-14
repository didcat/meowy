use crate::hir::FoundationType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Module {
    Core,
    Debug,
    Memory,
    Strings,
    Proof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    Type(FoundationType),
    Heap,
    StringCopy,
}

impl Module {
    pub(crate) fn resolve(name: &str) -> Option<Self> {
        match name {
            "core" => Some(Self::Core),
            "debug" => Some(Self::Debug),
            "memory" => Some(Self::Memory),
            "strings" => Some(Self::Strings),
            "proof" => Some(Self::Proof),
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
        }
    }

    pub(crate) fn partial(self) -> bool {
        matches!(self, Self::Memory | Self::Strings | Self::Proof)
    }

    pub(crate) fn item(self, name: &str) -> Option<Item> {
        match (self, name) {
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
        }
    }
}
