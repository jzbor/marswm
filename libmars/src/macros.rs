
#[macro_export]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + $crate::count!($($xs)*));
}

// Improved version of
// https://stackoverflow.com/a/64678145/10854888
#[macro_export]
macro_rules! enum_with_values {
    ($(#[$derives:meta])* $(vis $visibility:vis)? enum $name:ident { $($(#[$nested_meta:meta])* $member:ident),* }) => {
        $(#[$derives])*
        $($visibility)? enum $name {
            $($(#[$nested_meta])* $member),*
        }
        impl $name {
            #[allow(dead_code)]
            pub const VALUES: &'static [$name; $crate::count!($($member)*)] = &[$($name::$member,)*];
            #[allow(dead_code)]
            pub const SIZE: usize = $crate::count!($($member)*);
        }
    };
}
