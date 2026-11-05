mod json;
mod sha256;
mod engine;

use std::{env,fs,io::{self,Read},time::Instant,hint::black_box};
use json::{J,num,object,string};
use engine::{Model,apply,net_from_map};

fn benchmark(model:&Model, request:&J) -> Result<J,String> {
    let samples=request.get("samples")?.int()?;
    if samples!=1 && samples!=100 { return Err("Expected smoke or 100 samples".into()); }
    let generator=request.get("generator")?.int()? as i32;
    if generator!=1 { return Err("Benchmark generator must be frozen O01[0]".into()); }
    let word=model.expand(&model.normalize(request.get("long_word")?)?);
    let macros=request.get("macros")?.array()?.iter().map(|recipe| model.net(recipe)).collect::<Result<Vec<_>,_>>()?;
    let mut initial=model.ids.clone();
    let scramble=model.expand(&model.normalize(request.get("initial")?)?);
    model.apply_word(&mut initial,&scramble,&mut Vec::new());
    let mut rows=Vec::new();
    for metric in ["apply_generator","apply_long_word","compose_macros","net_effect"] {
        let mut times=Vec::new(); let mut digest=String::new();
        for _ in 0..samples {
            let mut labels=initial.clone(); let mut scratch=Vec::new();
            let start=Instant::now();
            let (elapsed,current)=match metric {
                "apply_generator" => {
                    let (s,d)=model.move_arrays(generator); apply(&mut labels,s,d,&mut scratch);
                    black_box(&labels); let elapsed=start.elapsed().as_nanos();
                    (elapsed,sha256::ints(&labels))
                },
                "apply_long_word" => {
                    model.apply_word(&mut labels,&word,&mut scratch);
                    black_box(&labels); let elapsed=start.elapsed().as_nanos();
                    (elapsed,sha256::ints(&labels))
                },
                "compose_macros" => {
                    let mut map=model.ids.clone();
                    for (s,d) in &macros { apply(&mut map,s,d,&mut scratch); }
                    let net=net_from_map(&map); black_box(&net); let elapsed=start.elapsed().as_nanos();
                    (elapsed,sha256::pair(&net.0,&net.1))
                },
                _ => {
                    let net=model.net(request.get("net_recipe")?)?;
                    black_box(&net); let elapsed=start.elapsed().as_nanos();
                    (elapsed,sha256::pair(&net.0,&net.1))
                },
            };
            if !digest.is_empty() && current!=digest { return Err("Benchmark output changed between samples".into()); }
            digest=current; times.push(num(elapsed));
        }
        rows.push(object([("metric",string(metric)),("ns",J::Array(times)),("output_sha256",string(digest))]));
    }
    Ok(object([("backend",string("rust")),("rows",J::Array(rows))]))
}

fn run() -> Result<(),String> {
    let args:Vec<String>=env::args().collect();
    if args.len()!=3 { return Err("Usage: lx6-port trace|bench|self-test model.bin (JSON on stdin)".into()); }
    let data=fs::read(&args[2]).map_err(|_| "Cannot read model")?;
    let model=Model::load(&data)?;
    if args[1]=="self-test" {
        sha256::self_test(); json::self_test(); model.self_test();
        println!("{}",object([("status",string("pass")),("model_sha256",string(sha256::hex(&data)))]).canonical());
        return Ok(());
    }
    let mut input=Vec::new(); io::stdin().read_to_end(&mut input).map_err(|_| "Cannot read input")?;
    let document=json::parse(&input)?;
    let output=match args[1].as_str() {
        "trace" => model.replay(&document)?, "bench" => benchmark(&model,&document)?,
        _ => return Err("Unknown mode".into()),
    };
    println!("{}",output.canonical()); Ok(())
}

fn main() {
    if let Err(error)=run() { eprintln!("LX-6: {error}"); std::process::exit(2); }
}
