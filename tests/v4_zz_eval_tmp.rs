// TEMPORARY evaluation harness for v4 algorithms (removed after the run).
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use codeaware_mcp::token_stats::estimate_tokens as bench_tokens;
use codeaware_mcp::v4::pack::{pack, PackCandidate, PackConfig, PackMode};
use codeaware_mcp::v4::summaries::{doc_head, read_capped, SummaryGenerator};
use codeaware_mcp::v4::{
    CandidateDiscovery, ContextRanker, DiscoveryConfig, RankSignals, RankingMode, SemanticContextAssembler,
    SemanticContextOptions, SemanticIndexBuilder,
};

thread_local! { static LAST_RICH: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new()); }
const K: usize = 5;
const BUDGET: usize = 4000;

#[derive(Default, Clone)]
struct Acc {
    tokens: Vec<f64>,
    recall: Vec<f64>,
    rich: Vec<f64>,
    covered: usize,
}

fn recall(selected: &[String], relevant: &BTreeSet<String>) -> f64 {
    let hit = relevant.iter().filter(|r| selected.contains(r)).count();
    hit as f64 / relevant.len() as f64
}

#[test]
fn eval_v4_variants() {
    let tasks_path = "/home/user/codeaware-mcp/benches/token_benchmark/tasks.json";
    let tasks: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(tasks_path).unwrap()).unwrap();
    let scratch = std::env::var("V4_EVAL_SCRATCH").unwrap();
    let mut results: HashMap<&'static str, Acc> = HashMap::new();
    let mut per_task = Vec::new();

    for task in tasks["tasks"].as_array().unwrap() {
        let id = task["id"].as_str().unwrap().to_string();
        let goal = task["goal"].as_str().unwrap().to_string();
        let relevant: BTreeSet<String> = task["relevant_files"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
        let root_buf = Path::new(&scratch).join("trees").join(&id);
        let root = root_buf.as_path();

        let discovered = CandidateDiscovery::discover_ranked(
            root, &goal, DiscoveryConfig { max_candidates: 100_000, ..Default::default() },
        ).unwrap();
        let paths: Vec<String> = discovered.iter().map(|c| c.path.clone()).collect();
        let mut contents: HashMap<String, String> = HashMap::new();
        let mut heads: HashMap<String, String> = HashMap::new();
        for p in &paths {
            if let Some(c) = read_capped(&root.join(p)) {
                heads.insert(p.clone(), doc_head(&c, 2));
                contents.insert(p.clone(), c);
            }
        }
        let paths: Vec<String> = paths.into_iter().filter(|p| contents.contains_key(p)).collect();
        let index = SemanticIndexBuilder::build(root).unwrap();
        let signals = RankSignals { index: Some(&index), doc_heads: Some(&heads) };

        let rank = |mode| ContextRanker::rank(mode, &goal, paths.clone(), signals);
        let legacy = rank(RankingMode::Legacy);
        let bm25 = rank(RankingMode::Bm25);
        let graph = rank(RankingMode::Bm25Graph);

        let full_topk = |ranked: &Vec<codeaware_mcp::v4::RankedPath>| {
            let sel: Vec<String> = ranked.iter().take(K).map(|r| r.path.clone()).collect();
            let tok: usize = sel.iter().map(|p| bench_tokens(&contents[p]) as usize).sum();
            (sel, tok)
        };
        let candidates = |ranked: &Vec<codeaware_mcp::v4::RankedPath>| -> Vec<PackCandidate> {
            ranked.iter().enumerate().map(|(i, r)| {
                let c = &contents[&r.path];
                PackCandidate {
                    id: format!("{i:06}"),
                    relevance: r.relevance,
                    full: Some(c.clone()),
                    skeleton: Some(SummaryGenerator::skeleton(&r.path, c)),
                    one_line: SummaryGenerator::one_line(&r.path, c),
                }
            }).collect()
        };
        let packed = |ranked: &Vec<codeaware_mcp::v4::RankedPath>, mode, gap: Option<f64>| {
            let cands = candidates(ranked);
            let cfg = PackConfig { mode, budget_tokens: BUDGET, early_stop_fraction: gap };
            let res = pack(&cfg, &cands);
            let mut sel = Vec::new();
            let mut rich = Vec::new();
            let mut tok = 0usize;
            for item in &res.items {
                let idx: usize = item.id.parse().unwrap();
                sel.push(ranked[idx].path.clone());
                if matches!(item.representation, codeaware_mcp::v4::Representation::Full(_) | codeaware_mcp::v4::Representation::Skeleton(_)) {
                    rich.push(ranked[idx].path.clone());
                }
                tok += bench_tokens(item.representation.text()) as usize;
            }
            LAST_RICH.with(|cell| *cell.borrow_mut() = rich);
            (sel, tok)
        };
        let mut record = |name: &'static str, sel: Vec<String>, tok: usize| {
            let r = recall(&sel, &relevant);
            let rich_sel = if name.contains("pack") || name.contains("knapsack") { LAST_RICH.with(|c| c.borrow().clone()) } else { sel.clone() };
            let acc = results.entry(name).or_default();
            acc.tokens.push(tok as f64);
            acc.recall.push(r);
            acc.rich.push(recall(&rich_sel, &relevant));
            if r > 0.0 { acc.covered += 1; }
            per_task.push(format!("{id}\t{name}\t{tok}\t{r:.2}"));
        };

        let (s, t) = full_topk(&legacy); record("A_legacy_top5_full", s, t);
        let (s, t) = full_topk(&bm25); record("B_bm25_top5_full", s, t);
        let (s, t) = full_topk(&graph); record("C_graph_top5_full", s, t);
        let (s, t) = packed(&legacy, PackMode::Legacy, None); record("D_legacy_prefix_pack4000", s, t);
        let (s, t) = packed(&legacy, PackMode::Knapsack, None); record("E_legacy_knapsack4000", s, t);
        let (s, t) = packed(&bm25, PackMode::Knapsack, None); record("F_bm25_knapsack4000", s, t);
        let (s, t) = packed(&graph, PackMode::Knapsack, None); record("G_graph_knapsack4000", s, t);
        let (s, t) = packed(&graph, PackMode::Knapsack, Some(0.3)); record("H_graph_knapsack_gap0.3", s, t);
        let (s, t) = packed(&bm25, PackMode::Knapsack, Some(0.3)); record("I_bm25_knapsack_gap0.3", s, t);

        // Semantic path (SemanticContextAssembler), default vs BM25 symbols + knapsack.
        let sem = |opts: SemanticContextOptions| {
            let items = SemanticContextAssembler::assemble(&goal, &index, opts);
            let sel: Vec<String> = items.iter().filter_map(|i| i.path.clone()).collect();
            let tok: usize = items.iter().map(|i| bench_tokens(&i.content) as usize).sum();
            (sel, tok)
        };
        let (s, t) = sem(SemanticContextOptions::default()); record("S_semantic_legacy", s, t);
        let (s, t) = sem(SemanticContextOptions { ranking: RankingMode::Bm25, ..Default::default() }); record("T_semantic_bm25", s, t);
        let (s, t) = sem(SemanticContextOptions {
            ranking: RankingMode::Bm25,
            pack: Some(PackConfig { mode: PackMode::Knapsack, budget_tokens: BUDGET, early_stop_fraction: Some(0.3) }),
            source_root: Some(root.to_path_buf()),
            ..Default::default()
        });
        record("U_semantic_bm25_knapsack_gap", s, t);
    }

    let mut names: Vec<&&str> = results.keys().collect();
    names.sort();
    let mut out = String::from("variant\tmean_tokens\ttotal_tokens\tmean_recall@5\trecall_full_or_skeleton\ttasks_with_any_hit\n");
    for name in names {
        let acc = &results[*name];
        let n = acc.tokens.len() as f64;
        out.push_str(&format!(
            "{name}\t{:.0}\t{:.0}\t{:.3}\t{:.3}\t{}\n",
            acc.tokens.iter().sum::<f64>() / n,
            acc.tokens.iter().sum::<f64>(),
            acc.recall.iter().sum::<f64>() / n,
            acc.rich.iter().sum::<f64>() / n,
            acc.covered
        ));
    }
    std::fs::write(format!("{scratch}/eval_summary.tsv"), &out).unwrap();
    std::fs::write(format!("{scratch}/eval_per_task.tsv"), per_task.join("\n")).unwrap();
    eprintln!("{out}");
}

#[test]
fn eval_delta_rereads() {
    use codeaware_mcp::v4::context::ServedMap;
    use codeaware_mcp::v4::Representation;
    let scratch = std::env::var("V4_EVAL_SCRATCH").unwrap();
    let repo = "/home/user/codeaware-mcp";
    let git = |args: &[&str]| -> String {
        let out = std::process::Command::new("git").args(args).current_dir(repo).output().unwrap();
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    let changed = git(&["log", "--format=", "--name-only", "-n", "80", "--", "*.rs"]);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for line in changed.lines().filter(|l| !l.is_empty()) {
        *counts.entry(line.to_string()).or_default() += 1;
    }
    let mut files: Vec<(String, usize)> = counts.into_iter().collect();
    files.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    files.truncate(8);

    let (mut full_tok, mut delta_tok) = (0usize, 0usize);
    let (mut n_full, mut n_diff, mut n_unch) = (0usize, 0usize, 0usize);
    let mut report = String::new();
    for (file, _) in &files {
        let shas: Vec<String> = git(&["log", "--format=%H", "-n", "40", "--reverse", "--", file])
            .lines().map(|s| s.to_string()).collect();
        let mut served = ServedMap::default();
        let (mut f_t, mut d_t) = (0usize, 0usize);
        for sha in &shas {
            let content = git(&["show", &format!("{sha}:{file}")]);
            // Model: each version is read twice (e.g. before and after an edit elsewhere).
            for _ in 0..2 {
                let rep = served.serve(file, &content);
                let full = bench_tokens(&content) as usize;
                f_t += full;
                d_t += bench_tokens(rep.text()) as usize;
                match rep {
                    Representation::Full(_) => n_full += 1,
                    Representation::Diff(_) => n_diff += 1,
                    Representation::Unchanged(_) => n_unch += 1,
                    _ => {}
                }
            }
        }
        report.push_str(&format!("{file}\tversions={}\tfull_tokens={f_t}\tdelta_tokens={d_t}\n", shas.len()));
        full_tok += f_t;
        delta_tok += d_t;
    }
    report.push_str(&format!(
        "TOTAL\tfull_tokens={full_tok}\tdelta_tokens={delta_tok}\tsaved={:.1}%\treps: full={n_full} diff={n_diff} unchanged={n_unch}\n",
        100.0 * (1.0 - delta_tok as f64 / full_tok as f64)
    ));
    std::fs::write(format!("{scratch}/eval_delta.tsv"), &report).unwrap();
    eprintln!("{report}");
}
