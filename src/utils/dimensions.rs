#![allow(dead_code)]

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct DimensionsPx {
    pub width: u32,
    pub height: u32,
}

impl DimensionsPx {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn to_nonzero(self) -> DimensionsPxNonzero {
        DimensionsPxNonzero {
            width: self.width.max(1),
            height: self.height.max(1),
        }
    }
}

impl From<DimensionsPxNonzero> for DimensionsPx {
    fn from(value: DimensionsPxNonzero) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

#[allow(clippy::cast_sign_loss)]
impl From<egui::Rect> for DimensionsPx {
    fn from(value: egui::Rect) -> Self {
        Self {
            width: value.width() as u32,
            height: value.height() as u32,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct DimensionsPxNonzero {
    width: u32,
    height: u32,
}

impl DimensionsPxNonzero {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
        }
    }

    pub fn width(self) -> u32 {
        self.width
    }

    pub fn height(self) -> u32 {
        self.height
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = width.max(1);
    }

    pub fn set_height(&mut self, height: u32) {
        self.height = height.max(1);
    }

    pub fn to_unconstrained(self) -> DimensionsPx {
        DimensionsPx {
            width: self.width,
            height: self.height,
        }
    }
}

impl From<egui::Rect> for DimensionsPxNonzero {
    fn from(value: egui::Rect) -> Self {
        DimensionsPx::from(value).to_nonzero()
    }
}
