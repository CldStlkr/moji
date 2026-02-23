#[macro_export]
macro_rules! styled_view {
    // Static styles: styled_view!(my_style, "text-red-500 font-bold");
    ($name:ident, $classes:expr) => {
        #[inline]
        pub fn $name() -> &'static str {
            $classes
        }
    };
    // Dynamic styles: styled_view!(my_style, arg: type, "base_classes", dynamic_expression)
    ($name:ident, $arg:ident: $type:ty, $base:expr, $dynamic:expr) => {
        #[inline]
        pub fn $name($arg: $type) -> String {
            format!("{} {}", $base, $dynamic)
        }
    };
}
