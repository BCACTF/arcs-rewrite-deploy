macro_rules! logging_parts {
    (
        $write_target:expr; <==
            $literal:literal-$expression:expr
            $(=> $sub_literal:literal-$sub_expression:expr)*
            $(=> * $ending_literal:literal)?
                $(
                    , $next_literal:literal-$next_expression:expr
                    $(=> $next_sub_literal:literal-$next_sub_expression:expr)*
                    $(=> * $next_ending_literal:literal)?
                )*
                    $(,)?
            
    ) => {
        {
            use {
                paste::paste,
                // std::io::Write,
            };
            logging_parts!(
                @impl $write_target; <== reduce_type: top; current_ident: b1;
                $literal-$expression
                    $(=> $sub_literal-$sub_expression)*
                    $(=> * $ending_literal)?
                        $(
                            , $next_literal-$next_expression
                            $(=> $next_sub_literal-$next_sub_expression)*
                            $(=> * $next_ending_literal)?
                        )*
                ; >-< [] []
            )
        }
        
    };
    
    (
        @impl $write_target:expr; <==
        reduce_type: top; current_ident: $current_ident:ident;
            $literal:literal-$expression:expr
            $(=> $sub_literal:literal-$sub_expression:expr)*
            $(=> * $ending_literal:literal)?
                $(
                    , $next_literal:literal-$next_expression:expr
                    $(=> $next_sub_literal:literal-$next_sub_expression:expr)*
                    $(=> * $next_ending_literal:literal)?
                )*;
            >-< [$($literals:literal)*] [$($idents:ident)*] 
    ) => {
        match $expression {
            Some($current_ident) => paste! {
                logging_parts!(
                    @impl $write_target; <== reduce_type: sub; current_ident: [<$current_ident _s>];
                    $(=> $sub_literal-$sub_expression)*
                    $(=> * $ending_literal)?;
                    >-< [$($literals)* $literal] [$($idents)* $current_ident];
                    next_up:
                        $(
                            $next_literal-$next_expression
                            $(=> $next_sub_literal-$next_sub_expression)*
                            $(=> * $next_ending_literal)?
                        ),*
                )
            },
            None => paste! {
                logging_parts!(
                    @impl $write_target; <== reduce_type: top; current_ident: [<$current_ident 1>];
                        $(
                            $next_literal-$next_expression
                            $(=> $next_sub_literal-$next_sub_expression)*
                            $(=> * $next_ending_literal)?
                        ),*;
                     >-< [$($literals)*] [$($idents)*]
                )
            },
        }
    };

    (
        @impl $write_target:expr; <==
        reduce_type: top; current_ident: $current_ident:ident; ;
        >-< [$($literals:literal)*] [$($idents:ident)*]
    ) => {
        $write_target.write_fmt(format_args!(concat!(concat!("", $($literals),*), "\n"), $($idents),*))
    };


    (
        @impl $write_target:expr; <==
            reduce_type: sub; current_ident: $current_ident:ident;
                => $sub_literal:literal - $sub_expression:expr $(=> $next_sub_literal:literal - $next_sub_expression:expr)*
                $(=> * $ending_literal:literal)?;
                >-< [$($literals:literal)+] [$($idents:ident)+];
                next_up:
                    $(
                        $next_next_literal:literal-$next_next_expression:expr
                        $(=> $next_next_sub_literal:literal-$next_next_sub_expression:expr)*
                        $(=> * $next_next_ending_literal:literal)?
                    ),*
    ) => {
        match $sub_expression {
            Some($current_ident) => paste!{
                logging_parts!(
                    @impl $write_target; <== reduce_type: sub; current_ident: [<$current_ident _s>];
                    $(=> $next_sub_literal-$next_sub_expression)* $(=> * $ending_literal)?;
                    >-< [$($literals)+ $sub_literal] [$($idents)+ $current_ident];
                    next_up:
                        $(
                            $next_next_literal-$next_next_expression
                            $(=> $next_next_sub_literal-$next_next_sub_expression)*
                            $(=> * $next_next_ending_literal)?
                        ),*
                )
            },
            None => logging_parts!(
                @impl $write_target; <== reduce_type: sub; current_ident: $current_ident;
                $(=> * $ending_literal)?;
                >-< [$($literals)+] [$($idents)*];
                next_up:
                    $(
                        $next_next_literal-$next_next_expression
                        $(=> $next_next_sub_literal-$next_next_sub_expression)*
                        $(=> * $next_next_ending_literal)?
                    ),*
            ),
        }
    };

    (
        @impl $write_target:expr; <==
            reduce_type: sub; current_ident: $current_ident:ident; => * $ending_literal:literal;
            >-< [$($literals:literal)+] [$($idents:ident)+];
            next_up:
                $(
                    $next_literal:literal-$next_expression:expr
                    $(=> $next_sub_literal:literal-$next_sub_expression:expr)*
                    $(=> * $next_ending_literal:literal)?
                ),*
    ) => {
        logging_parts!(
            @impl $write_target; <==
                reduce_type: sub; current_ident: $current_ident; ;
                >-< [$($literals)+ $ending_literal] [$($idents)+];
                next_up:
                    $(
                        $next_literal-$next_expression
                        $(=> $next_sub_literal-$next_sub_expression)*
                        $(=> * $next_ending_literal)?
                    ),*
        )
    };

    (
        @impl $write_target:expr; <==
            reduce_type: sub; current_ident: $current_ident:ident; ;
            >-< [$($literals:literal)*] [$($idents:ident)*]; 
            next_up:
                $(
                    $next_literal:literal-$next_expression:expr
                    $(=> $next_sub_literal:literal-$next_sub_expression:expr)*
                    $(=> * $next_ending_literal:literal)?
                ),*
    ) => {
        paste! {
            logging_parts!(
                @impl $write_target; <== reduce_type: top; current_ident: [<$current_ident _b1>];
                    $(
                        $next_literal-$next_expression
                        $(=> $next_sub_literal-$next_sub_expression)*
                        $(=> * $next_ending_literal)?
                    ),*;
                 >-< [$($literals)*] [$($idents)*]
            )
        }
    };
}

pub (crate) use logging_parts;