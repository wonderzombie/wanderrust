#[macro_export]
macro_rules! enum_with_str {
    ( $enum_name:ident, [ $( $variant:ident ),* ] ) => {
        #[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash, Reflect)]
        pub enum $enum_name {
            #[default]
            Unset,
            $( $variant, )*
        }

        #[allow(dead_code)]
        impl $enum_name {
            pub fn all() -> &'static [$enum_name] {
                &[ $( $enum_name::$variant, )* ]
            }

            pub fn pairs() -> &'static [(&'static str, $enum_name)] {
                &[ $( (stringify!($variant), $enum_name::$variant), )* ]
            }

            pub fn from_str(value: &str) -> Option<$enum_name> {
                Self::pairs().iter().find(|(s, _)| &value == s).copied().map(|(_, v)| v)
            }
        }
    };
}
