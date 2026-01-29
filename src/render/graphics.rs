use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_graphics::context::CGContext;
use core_text::font::CTFont;
use foreign_types::ForeignType;

use crate::config::parse_hex_color;

pub struct Graphics {
    background_color: (f64, f64, f64, f64),
    text_color: (f64, f64, f64, f64),
    font: CTFont,
}

impl Graphics {
    pub fn new(bg_color: &str, text_color: &str, font_family: &str, font_size: f64) -> Self {
        let background_color = parse_hex_color(bg_color).unwrap_or((0.1, 0.1, 0.15, 1.0));
        let text_color = parse_hex_color(text_color).unwrap_or((0.8, 0.85, 0.95, 1.0));

        let font = core_text::font::new_from_name(font_family, font_size).unwrap_or_else(|_| {
            log::warn!("Failed to load font '{}', using Helvetica", font_family);
            core_text::font::new_from_name("Helvetica", font_size)
                .expect("Failed to load fallback font")
        });

        Self {
            background_color,
            text_color,
            font,
        }
    }

    pub fn draw_background(&self, ctx: &mut CGContext, width: f64, height: f64) {
        let (r, g, b, a) = self.background_color;
        ctx.set_rgb_fill_color(r, g, b, a);
        ctx.fill_rect(core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(0.0, 0.0),
            &core_graphics::geometry::CGSize::new(width, height),
        ));
    }

    pub fn draw_text(&self, ctx: &mut CGContext, text: &str, x: f64, y: f64) {
        use core_foundation::attributed_string::CFMutableAttributedString;
        use core_foundation::base::CFRange;
        use core_text::line::CTLine;
        use core_text::string_attributes::kCTFontAttributeName;

        let (r, g, b, a) = self.text_color;

        // Create attributed string
        let cf_string = CFString::new(text);
        let mut attr_string = CFMutableAttributedString::new();
        attr_string.replace_str(&cf_string, CFRange::init(0, 0));

        let range = CFRange::init(0, text.len() as isize);

        // Set font attribute using the kCTFontAttributeName constant
        unsafe {
            // kCTFontAttributeName is a CFStringRef (pointer)
            attr_string.set_attribute(range, kCTFontAttributeName, &self.font);
        }

        // Create CTLine and draw
        let line = CTLine::new_with_attributed_string(attr_string.as_concrete_TypeRef());

        // Set text color
        let color = core_graphics::color::CGColor::rgb(r, g, b, a);
        ctx.set_fill_color(&color);

        let identity = core_graphics::geometry::CGAffineTransform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx: 0.0,
            ty: 0.0,
        };
        ctx.set_text_matrix(&identity);
        ctx.set_text_position(x, y);

        // Draw the line
        unsafe {
            use core_text::line::CTLineRef;
            unsafe extern "C" {
                fn CTLineDraw(line: CTLineRef, context: core_graphics::sys::CGContextRef);
            }
            CTLineDraw(line.as_concrete_TypeRef(), ctx.as_ptr());
        }
    }

    pub fn measure_text(&self, text: &str) -> f64 {
        use core_foundation::attributed_string::CFMutableAttributedString;
        use core_foundation::base::CFRange;
        use core_text::line::CTLine;
        use core_text::string_attributes::kCTFontAttributeName;

        let cf_string = CFString::new(text);
        let mut attr_string = CFMutableAttributedString::new();
        attr_string.replace_str(&cf_string, CFRange::init(0, 0));

        let range = CFRange::init(0, text.len() as isize);

        unsafe {
            attr_string.set_attribute(range, kCTFontAttributeName, &self.font);
        }

        let line = CTLine::new_with_attributed_string(attr_string.as_concrete_TypeRef());
        line.get_typographic_bounds().width
    }

    pub fn font_height(&self) -> f64 {
        self.font.ascent() + self.font.descent()
    }

    pub fn font_ascent(&self) -> f64 {
        self.font.ascent()
    }

    pub fn font_descent(&self) -> f64 {
        self.font.descent()
    }
}
