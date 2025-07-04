use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Config {
    help: bool,
    inplace: bool,
    no_coretemp: bool,
    no_nvme: bool,
    rc_file: Option<String>,
}

impl Config {
    fn print_help(prog: &String) {
        println!("\
{prog} [-h] [-i|--inplace] [options] [RC]

  Update hwmon path in conky rc file.

Arguments:
  RC            : conky rc file.

Options:
  -h, --help    : print help document.
  -i, --inplace : inpalce RC file.
  --no-coretemp : do not update coretemp path.
  --no-nvme     : do not update nvme (disk) path.
");
    }

    fn from_cli(args: &[String]) -> Config {
        let mut i = 1;
        let mut help = false;
        let mut inplace = false;
        let mut no_coretemp = false;
        let mut no_nvme = false;
        let mut rc_file = None;

        while i < args.len() {
            let arg = args[i].as_str();
            match arg {
                "-h" | "--help" => {
                    help = true;
                    i += 1;
                }
                "-i" | "--inplace" => {
                    inplace = true;
                    i += 1;
                }
                "--no-coretemp" => {
                    no_coretemp = true;
                    i += 1;
                }
                "--no-nvme" => {
                    no_nvme = true;
                    i += 1;
                }
                arg => {
                    match rc_file {
                        None => {
                            rc_file = Some(String::from(arg));
                            i += 1;
                        }
                        Some(_) => {
                            panic!("multiple rc files");
                        }
                    };
                }
            };
        }

        Config { help, inplace, no_coretemp, no_nvme, rc_file }
    }

    fn with_default_rc_file(&mut self) {
        if self.rc_file.is_none() {
            let home = env::var("HOME").unwrap();
            let mut path = PathBuf::from(home);
            path.push(".conkyrc");
            self.rc_file = Some(String::from(path.to_str().unwrap()));
        }
    }
}

fn find_hwmon_path(name: &'static str) -> Option<i32> {
    let mut i = 0;
    loop {
        let path_str = format!("/sys/class/hwmon/hwmon{i}/name");
        let path = Path::new(&path_str);
        if !path.is_file() {
            return None;
        } else {
            if let Ok(read) = fs::read_to_string(path) {
                if read == name {
                    return Some(i);
                }
            }
            i += 1;
        }
    }
}

fn replace_hwmon_path(line: String, mon: Option<i32>) -> String {
    if mon.is_none() {
        return line;
    }

    let mon = format!("{0}", mon.unwrap());
    let mut p = 0;
    let mut ret = String::new();

    while p < line.len() {
        if let Some(ii) = line[p..].find(" hwmon ") {
            let i = p + ii;
            let mut s = 7;

            loop {
                match line[i + s..].find(" ") {
                    None => {
                        ret.push_str(&line[p..i + s]);
                        p = i + s;
                        break;
                    }
                    Some(0) => s += 1,
                    Some(jj) => {
                        ret.push_str(&line[p..i + s]);

                        let j = i + s + jj;
                        let num = &line[i + s..j];
                        if num.parse::<u32>().is_ok() {
                            ret.push_str(&mon);
                        } else {
                            ret.push_str(num);
                        }
                        p = j;
                        break;
                    }
                }
            }

        } else {
            ret.push_str(&line[p..]);
            break;
        }
    }

    ret
}

fn update_conkyrc_content<W: Write>(config: &Config, input: &String, output: &mut W) -> io::Result<()> {
    let rc_file = File::open(input)?;
    let reader = BufReader::new(rc_file);

    for line_read in reader.lines() {
        let line = line_read?;
        if let Some(0) = line.find("--") {
            writeln!(output, "{line}")?;
        } else if let None = line.find("hwmon") {
            writeln!(output, "{line}")?;
        } else if let Some(0) = line.find("== CPU ==") && !config.no_coretemp {
            let updated = replace_hwmon_path(String::from(&line), find_hwmon_path("coretemp"));
            writeln!(output, "{updated}")?;
        } else if let Some(0) = line.find("== Disk IO ==") && !config.no_nvme {
            let updated = replace_hwmon_path(String::from(&line), find_hwmon_path("nvme"));
            writeln!(output, "{updated}")?;
        } else {
            writeln!(output, "{line}")?;
        }
    }

    Ok(())

}

fn update_conkyrc_file(config: &Config, rc_file: &String) -> io::Result<()> {
    if config.inplace {
        let path = Path::new("/tmp/.conkyrc");
        let mut file = File::create(&path)?;
        if let Ok(()) = update_conkyrc_content(config, rc_file, &mut file) {
            fs::rename(path, rc_file)?;
        }
        let _ = fs::remove_file(&path);

    } else {
        let mut stdout = io::stdout();
        update_conkyrc_content(config, rc_file, &mut stdout)?;
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut config = Config::from_cli(&args);

    if config.help {
        Config::print_help(&args[0]);
    } else {
        config.with_default_rc_file();
        // println!("rc_file = {0}", config.rc_file.unwrap());

        let ref rc_file = config.rc_file.take().unwrap();
        update_conkyrc_file(&config, &rc_file).unwrap_or_else(|err| {
            panic!("{err}");
        });
    }
}
