use std::fmt::{Display, Debug};
pub use paste::paste;
pub use lazy_static::lazy_static;

#[macro_export]
macro_rules! env_var_req {
    ($env_name:ident $(-> $short_name:ident)?) => {
        env_var_req!(impl $env_name, $($short_name)? $env_name);
    };
    (impl $env_name:ident, $var_name:ident $($extra:ident)?) => {
        $crate::paste! {
            $crate::lazy_static! {
                pub static ref $var_name: Result<String, &'static str> = std::env::var(stringify!($env_name)).map_err(|_| stringify!($env_name));
            }
            pub fn [<$var_name:lower>]() -> &'static str {
                $var_name.as_ref().unwrap()
            }
        }
    }
}

#[macro_export]
macro_rules! assert_req_env {
    ($check_name_fn:ident: $($names:ident),+) => {
        const NAMES_LEN: usize = [$(stringify!($names)),+].len();
        pub fn $check_name_fn() -> Result<(), EnvVarErr<NAMES_LEN>> {
            let errs = [$($names.as_ref().err().map(|e| *e)),+];
            if errs.iter().any(Option::is_some) {
                Err(EnvVarErr::new(errs))
            } else {
                Ok(())
            }
        }
    };
}

pub struct EnvVarErr<const T: usize>([Option<&'static str>; T]);

impl<const T: usize> EnvVarErr<T> {
    pub fn new(inner: [Option<&'static str>; T]) -> Self {
        Self(inner)
    }
}
impl<const T: usize> Display for EnvVarErr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0
            .into_iter()
            .filter_map(|a| a)
            .enumerate()
            .map(
                |(idx, var_name)| if idx == 0 {
                    write!(f, "{var_name}")
                } else {
                    write!(f, ", {var_name}")
                }
            )
            .collect()
    }
}
impl<const T: usize> Debug for EnvVarErr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
