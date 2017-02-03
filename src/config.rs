extern crate yaml_rust;

use std::error::Error;
use std::path::Path;
use util;
use self::yaml_rust::{YamlLoader, Yaml};

// I may have gotten a little carried away with these macros..

// Due to limitations of the macro system and the YAML library, we must use a generic solution
// to retrieve values dynamically.
fn yaml_to_string(yaml: &Yaml) -> Option<String> {
    use self::yaml_rust::Yaml::*;

    match *yaml {
        Real(ref string) | String(ref string) =>
            Some(string.clone()),
        Integer(num) => Some(format!("{}", num)),
        _ => None,
    }
}

macro_rules! gen_value {
    // Value type
    (v $parent:expr, $disp_name:expr, $default:expr) => {{
        yaml_to_string(&$parent[$disp_name])
            .and_then(|s| s.parse().ok())
            .unwrap_or($default)
    }};

    // Value type with minimum
    (vmin $parent:expr, $disp_name:expr, [$min:expr, $default:expr]) => {{
        let result = gen_value!(v $parent, $disp_name, $default);
        if result < $min { $min } else { result }
    }};

    // Optional type
    (o $parent:expr, $disp_name:expr, $default:expr) => {{
        yaml_to_string(&$parent[$disp_name])
            .and_then(|s| s.parse().ok())
    }};

    // Optional type with minimum
    (omin $parent:expr, $disp_name:expr, [$min:expr, $default:expr]) => {{
        let result = gen_value!(o $parent, $disp_name, $default);
        result.map(|v| if v < $min { $min } else { v })
    }};

    // Struct type
    ($t:ident $parent:expr, $disp_name:expr, $default:expr) => {{
        $t::parse(&$parent)
    }};
}

macro_rules! create_sub_config {
    ($yaml_name:expr, $name:ident,
        $($opt:ident $field:ident.$field_type:ty => $disp_name:expr => $default:tt,)+) => {

        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $field_type,)+
        }

        impl $name {
            fn new(doc: &Yaml) -> $name {
                let parent = &doc[$yaml_name];

                $name {
                    $($field: gen_value!($opt parent, $disp_name, $default),)+
                }
            }
        }
    };
}

macro_rules! try_opt {
    ($value:expr) => {{
        match $value {
            Some(v) => v,
            None    => return None,
        }
    }};
}

macro_rules! create_arr_struct {
    ($yaml_name:expr, $name:ident, $($field:ident.$field_type:ty => $disp_name:expr,)+) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $field: $field_type,)+
        }

        impl $name {
            pub fn parse(doc: &Yaml) -> Vec<$name> {
                let parent = &doc[$yaml_name];

                parent.as_vec()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(|field| {
                        Some($name {
                            $($field:
                                try_opt!(
                                    yaml_to_string(&field[$disp_name])
                                    .and_then(|s| s.parse().ok())
                                ),)+
                        })
                    })
                    .collect()
            }
        }
    };
}

create_arr_struct!("Feed Percentages", FeedPercentage,
    name.String => "Name",
    spike.f32   => "Spike",
);

create_sub_config!("Global", Global,
    vmin spike.f32                   => "Spike"                                => [0.0, 0.20],
    vmin unskewed_reset_pcnt.f32     => "Unskewed Reset Percentage"            => [0.0, 0.15],
    vmin unskewed_adjust_pcnt.f32    => "Unskewed Adjust Percentage"           => [0.0, 0.01],
    vmin low_listener_increase.f32   => "Low Listener Increase Percentage"     => [0.0, 0.005],
    vmin high_listener_dec.f32       => "High Listener Decrease Percentage"    => [0.0, 0.02],
    vmin high_listener_dec_every.f32 => "High Listener Decrease Per Listeners" => [1.0, 100.0],
    vmin update_time.f32             => "Update Time"                          => [5.0, 6.0],
    v minimum_listeners.u32          => "Minimum Listeners"                    => 15,
    o state_feeds_id.Option<u8>      => "State Feeds ID"                       => [0, None],
);

#[derive(Debug)]
pub struct Config {
    pub global:           Global,
    pub feed_percentages: Vec<FeedPercentage>,
    pub blacklist:        Vec<String>,
}

pub fn load_from_file(path: &Path) -> Result<Config, Box<Error>> {
    let doc = YamlLoader::load_from_str(&util::read_file(path)?)?;
    let doc = &doc[0]; // We don't care about multiple documents

    Ok(Config {
        global:           Global::new(&doc),
        feed_percentages: FeedPercentage::parse(&doc),
        blacklist:        Vec::new(),
    })
}