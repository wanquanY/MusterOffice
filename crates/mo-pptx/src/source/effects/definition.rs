use super::read::{self, Children, Slots};
use super::*;
use crate::source::{
    drawingml::*,
    fill::{NativeFillAlignment, SourceFill},
    malformed,
};
use mo_common::Emu;
use mo_xml::{Element, XmlError};

// One inventory drives typed payloads, attribute decoding and child grammar.
// The box holds only the actual payload, rather than the largest possible
// effect at every graph node. Empty payloads have no heap allocation.
macro_rules! effects {
    ($($kind:ident($payload:ident, $xml:literal, $children:expr) {
        $($field:ident: $ty:ty => $attribute:literal, $parse:ident;)*
    } { $($child:ident: $child_ty:ty => $take:ident;)* })*) => {
        $(#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all="camelCase", deny_unknown_fields)]
        pub struct $payload { $(pub $field: $ty,)* $(pub $child: $child_ty,)* })*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(tag="kind", rename_all="camelCase", deny_unknown_fields)]
        pub enum SourceEffectDefinition { $($kind(Box<$payload>),)* }
        pub(in crate::source) enum PendingEffect { $($kind { $($field:$ty,)* },)* }
        impl PendingEffect {
            pub fn read(name:&str, e:&Element)->Result<(Self,&'static [&'static str]),XmlError> {
                match name { $($xml=>Ok((Self::$kind { $($field:read::$parse(e,$attribute)?,)* }, &[$($attribute,)*])),)*
                    _=>Err(malformed("unknown native effect")) }
            }
            pub fn children(&self)->Children { match self { $(Self::$kind{..}=>$children,)* } }
            pub fn finish(self, mut slots:Slots)->Result<SourceEffectDefinition,XmlError> {
                let result=match self { $(Self::$kind{$($field,)*} => SourceEffectDefinition::$kind(Box::new($payload {$($field,)* $($child:slots.$take()?,)*})),)* };
                slots.require_empty()?;
                Ok(result)
            }
        }
    };
}
effects! {
    Container(SourceEffectContainer,"cont",Children::Effects) {
        container_type:Option<NativeEffectContainerType> =>"type",optional_enum;
        name:Option<String> =>"name",optional_token;
    } { nodes:Vec<u32> =>take_nodes; }
    Reference(SourceEffectLink,"effect",Children::None) {
        reference:String=>"ref",token;
    } {}
    AlphaBiLevel(SourceAlphaBiLevel,"alphaBiLevel",Children::None) {
        threshold:NativePercentage=>"thresh",required_positive_fixed;
    } {}
    AlphaCeiling(SourceAlphaCeiling,"alphaCeiling",Children::None) {} {}
    AlphaFloor(SourceAlphaFloor,"alphaFloor",Children::None) {} {}
    AlphaInverse(SourceAlphaInverse,"alphaInv",Children::OptionalColor) {} { color:Option<SourceColor> =>take_optional_color; }
    AlphaModulate(SourceAlphaModulate,"alphaMod",Children::Container) {} { container:u32=>take_node; }
    AlphaModulateFixed(SourceAlphaModulateFixed,"alphaModFix",Children::None) {
        amount:Option<NativePercentage> =>"amt",optional_positive;
    } {}
    AlphaOutset(SourceAlphaOutset,"alphaOutset",Children::None) {
        radius:Option<NativeCoordinate> =>"rad",optional_coordinate;
    } {}
    AlphaReplace(SourceAlphaReplace,"alphaRepl",Children::None) {
        alpha:NativePercentage=>"a",required_positive_fixed;
    } {}
    BiLevel(SourceBiLevel,"biLevel",Children::None) {
        threshold:NativePercentage=>"thresh",required_positive_fixed;
    } {}
    Blend(SourceBlend,"blend",Children::Container) {
        blend:NativeBlendMode=>"blend",required_enum;
    } { container:u32=>take_node; }
    Blur(SourceBlur,"blur",Children::None) {
        radius:Option<Emu> =>"rad",optional_positive_coordinate;
        grow:Option<bool> =>"grow",optional_boolean;
    } {}
    ColorChange(SourceColorChange,"clrChange",Children::ColorChange) {
        use_alpha:Option<bool> =>"useA",optional_boolean;
    } { from:SourceColor=>take_color; to:SourceColor=>take_color; }
    ColorReplace(SourceColorReplace,"clrRepl",Children::Color) {} { color:SourceColor=>take_color; }
    Duotone(SourceDuotone,"duotone",Children::TwoColors) {} { first:SourceColor=>take_color; second:SourceColor=>take_color; }
    Fill(SourceFillEffect,"fill",Children::Fill) {} { fill:Box<SourceFill> =>take_fill; }
    FillOverlay(SourceFillOverlay,"fillOverlay",Children::Fill) {
        blend:NativeBlendMode=>"blend",required_enum;
    } { fill:Box<SourceFill> =>take_fill; }
    Glow(SourceGlow,"glow",Children::Color) {
        radius:Option<Emu> =>"rad",optional_positive_coordinate;
    } { color:SourceColor=>take_color; }
    Grayscale(SourceGrayscale,"grayscl",Children::None) {} {}
    Hsl(SourceHslEffect,"hsl",Children::None) {
        hue:Option<u32> =>"hue",optional_positive_angle;
        saturation:Option<NativePercentage> =>"sat",optional_fixed;
        luminance:Option<NativePercentage> =>"lum",optional_fixed;
    } {}
    InnerShadow(SourceInnerShadow,"innerShdw",Children::Color) {
        blur_radius:Option<Emu> =>"blurRad",optional_positive_coordinate;
        distance:Option<Emu> =>"dist",optional_positive_coordinate;
        direction:Option<u32> =>"dir",optional_positive_angle;
    } { color:SourceColor=>take_color; }
    Luminance(SourceLuminanceEffect,"lum",Children::None) {
        brightness:Option<NativePercentage> =>"bright",optional_fixed;
        contrast:Option<NativePercentage> =>"contrast",optional_fixed;
    } {}
    OuterShadow(SourceOuterShadow,"outerShdw",Children::Color) {
        blur_radius:Option<Emu> =>"blurRad",optional_positive_coordinate;
        distance:Option<Emu> =>"dist",optional_positive_coordinate;
        direction:Option<u32> =>"dir",optional_positive_angle;
        scale_x:Option<NativePercentage> =>"sx",optional_percentage;
        scale_y:Option<NativePercentage> =>"sy",optional_percentage;
        skew_x:Option<i32> =>"kx",optional_fixed_angle;
        skew_y:Option<i32> =>"ky",optional_fixed_angle;
        alignment:Option<NativeFillAlignment> =>"algn",optional_enum;
        rotate_with_shape:Option<bool> =>"rotWithShape",optional_boolean;
    } { color:SourceColor=>take_color; }
    PresetShadow(SourcePresetShadow,"prstShdw",Children::Color) {
        preset:NativePresetShadow=>"prst",required_enum;
        distance:Option<Emu> =>"dist",optional_positive_coordinate;
        direction:Option<u32> =>"dir",optional_positive_angle;
    } { color:SourceColor=>take_color; }
    Reflection(SourceReflection,"reflection",Children::None) {
        blur_radius:Option<Emu> =>"blurRad",optional_positive_coordinate;
        start_alpha:Option<NativePercentage> =>"stA",optional_positive_fixed;
        start_position:Option<NativePercentage> =>"stPos",optional_positive_fixed;
        end_alpha:Option<NativePercentage> =>"endA",optional_positive_fixed;
        end_position:Option<NativePercentage> =>"endPos",optional_positive_fixed;
        distance:Option<Emu> =>"dist",optional_positive_coordinate;
        direction:Option<u32> =>"dir",optional_positive_angle;
        fade_direction:Option<u32> =>"fadeDir",optional_positive_angle;
        scale_x:Option<NativePercentage> =>"sx",optional_percentage;
        scale_y:Option<NativePercentage> =>"sy",optional_percentage;
        skew_x:Option<i32> =>"kx",optional_fixed_angle;
        skew_y:Option<i32> =>"ky",optional_fixed_angle;
        alignment:Option<NativeFillAlignment> =>"algn",optional_enum;
        rotate_with_shape:Option<bool> =>"rotWithShape",optional_boolean;
    } {}
    RelativeOffset(SourceRelativeOffset,"relOff",Children::None) {
        translate_x:Option<NativePercentage> =>"tx",optional_percentage;
        translate_y:Option<NativePercentage> =>"ty",optional_percentage;
    } {}
    SoftEdge(SourceSoftEdge,"softEdge",Children::None) {
        radius:Emu=>"rad",positive_coordinate;
    } {}
    Tint(SourceTint,"tint",Children::None) {
        hue:Option<u32> =>"hue",optional_positive_angle;
        amount:Option<NativePercentage> =>"amt",optional_fixed;
    } {}
    Transform(SourceEffectTransform,"xfrm",Children::None) {
        scale_x:Option<NativePercentage> =>"sx",optional_percentage;
        scale_y:Option<NativePercentage> =>"sy",optional_percentage;
        skew_x:Option<i32> =>"kx",optional_fixed_angle;
        skew_y:Option<i32> =>"ky",optional_fixed_angle;
        translate_x:Option<NativeCoordinate> =>"tx",optional_coordinate;
        translate_y:Option<NativeCoordinate> =>"ty",optional_coordinate;
    } {}
}
