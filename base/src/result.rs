use crate::error::LoadumError;

pub type LoadumResult<T> = Result<T, LoadumError>;

#[macro_export]
macro_rules! context {
    ($fmt:expr $(, $($args:expr),+)? => $($stmts:stmt)+) => {
        (|| {
            $($stmts)+
        })().map_err(|e| $crate::error::LoadumError::from(e).context(format!(concat!("Failed to ",$fmt) $(, $($args),+)?)))
    };
}

#[cfg(test)]
mod tests {
    use crate::context;
    use crate::error::{LoadumError, bail};
    use crate::result::LoadumResult;
    use std::env::set_var;
    use std::num::ParseFloatError;
    use std::str::FromStr;

    #[test]
    fn test_without_macro() {
        let result: LoadumResult<u32> = (|| {
            bail!("foo");
            //Err(std::io::Error::new(std::io::ErrorKind::NotFound, "foo"))
        })()
        .map_err(|e| LoadumError::from(e).context("bar"));
        let _err = result.unwrap_err();
        //println!("Error: {:?}", _err);
    }

    #[test]
    fn test_context_macro_ok() {
        let _result = {
            context!("grok stuff for {}", "bar" =>
                Ok::<i32, std::io::Error>(0)
            )
        }
        .unwrap();
    }

    #[test]
    fn test_context_macro_err() {
        unsafe { set_var("RUST_BACKTRACE", "1") };
        fn my_broken_function() -> LoadumResult<u32> {
            bail!("ungrokkable");
        }
        let result = {
            context!("grok stuff for {}", "bar" => {
                my_broken_function()
            })
        }
        .expect_err("Should have errored, but was");
        assert_eq!("Failed to grok stuff for bar", result.to_string());
        assert!(format!("{:?}", result).contains("my_broken_function"));
    }

    #[test]
    fn test_context_macro_err2() {
        fn my_broken_function() -> Result<f32, ParseFloatError> {
            f32::from_str("xyz")
        }
        let result = {
            context!("grok stuff for {}", "bar" => {
                my_broken_function()
            })
        }
        .expect_err("Should have errored, but was");
        assert_eq!("Failed to grok stuff for bar", result.to_string());
    }
}
