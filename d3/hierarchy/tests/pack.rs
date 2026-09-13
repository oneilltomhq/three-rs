//! Ports of d3-hierarchy/test/pack/{enclose,siblings,deterministic,flare}-test.js.
//!
//! Skipped, as noted in the crate README: bench-enclose.js (a benchmark),
//! find-bugs.js, find-enclose-bugs.js, find-place-bugs.js (random fuzzers).

mod common;
use d3_hierarchy::pack::{enclose, siblings, Circle, Pack};
use d3_hierarchy::{hierarchy, Datum, Tree};
use serde_json::json;

// ----------------------------------------------------------- enclose-test.js

// https://github.com/d3/d3-hierarchy/issues/188
#[test]
fn pack_enclose_circles_handles_a_tricky_case() {
    assert_eq!(
        enclose(&[
            Circle::new(14.5, 48.5, 7.585),
            Circle::new(9.5, 79.5, 2.585),
            Circle::new(15.5, 73.5, 8.585),
        ]),
        Some(Circle {
            r: 20.790781637717107,
            x: 12.80193548387092,
            y: 61.59615384615385,
        })
    );
}

// ---------------------------------------------------------- siblings-test.js

fn circle_value(value: f64) -> Circle {
    Circle::radius(value.sqrt())
}

fn circle_radius(radius: f64) -> Circle {
    Circle::radius(radius)
}

fn values(values: &[f64]) -> Vec<Circle> {
    values.iter().map(|&v| circle_value(v)).collect()
}

fn radii(radii: &[f64]) -> Vec<Circle> {
    radii.iter().map(|&r| circle_radius(r)).collect()
}

fn permute(array: &mut Vec<Circle>, f: &mut impl FnMut(&mut Vec<Circle>), n: usize) {
    if n == 1 {
        f(array);
        return;
    }
    for i in 0..n - 1 {
        permute(array, f, n - 1);
        let a = if n & 1 != 0 { 0 } else { i };
        array.swap(a, n - 1);
    }
    permute(array, f, n - 1);
}

fn intersects_any(circles: &[Circle]) -> bool {
    let n = circles.len();
    for i in 0..n {
        let ci = circles[i];
        for j in i + 1..n {
            if intersects(&ci, &circles[j]) {
                return true;
            }
        }
    }
    false
}

fn intersects(a: &Circle, b: &Circle) -> bool {
    let dr = a.r + b.r - 1e-6;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    dr > 0.0 && dr * dr > dx * dx + dy * dy
}

#[test]
fn pack_siblings_circles_produces_a_non_overlapping_layout_of_circles() {
    let mut check = |p: &mut Vec<Circle>| {
        siblings(p);
        if intersects_any(p) {
            panic!("{:?}", p.iter().map(|c| c.r).collect::<Vec<_>>());
        }
    };
    let mut a = values(&[100.0, 200.0, 500.0, 70.0, 3.0]);
    let n = a.len();
    permute(&mut a, &mut check, n);
    let mut a = values(&[3.0, 30.0, 50.0, 400.0, 600.0]);
    let n = a.len();
    permute(&mut a, &mut check, n);
    let mut a = values(&[1.0, 1.0, 3.0, 30.0, 50.0, 400.0, 600.0]);
    let n = a.len();
    permute(&mut a, &mut check, n);

    let mut c = radii(&[
        0.24155803737254639,
        0.06349736576607135,
        0.4721808601742349,
        0.7469141449305542,
        1.6399276349079663,
    ]);
    siblings(&mut c);
    assert_eq!(intersects_any(&c), false);

    let mut c = values(&[
        2.0, 9071.0, 79.0, 51.0, 325.0, 867.0, 546.0, 19773.0, 371.0, 16.0, 165781.0, 10474.0,
        6928.0, 40201.0, 31062.0, 14213.0, 8626.0, 12.0, 299.0, 1075.0, 98918.0, 4738.0, 664.0,
        2694.0, 2619.0, 51237.0, 21431.0, 99.0, 5920.0, 1117.0, 321.0, 519162.0, 33559.0, 234.0,
        4207.0,
    ]);
    siblings(&mut c);
    assert_eq!(intersects_any(&c), false);

    let mut c = radii(&[
        0.3371386860049076,
        58.65337373332081,
        2.118883785686244,
        1.7024669121097333,
        5.834919697833051,
        8.949453403094978,
        6.792586534702093,
        105.30490014617664,
        6.058936212213754,
        0.9535722042975694,
        313.7636051642043,
    ]);
    siblings(&mut c);
    assert_eq!(intersects_any(&c), false);

    let mut c = radii(&[
        6.26551789195159,
        1.707773433636342,
        9.43220282933871,
        9.298909705475646,
        5.753163715613753,
        8.882383159012575,
        0.5819319661882536,
        2.0234859171687747,
        2.096171518434433,
        9.762727931304937,
    ]);
    siblings(&mut c);
    assert_eq!(intersects_any(&c), false);

    let mut c = radii(&[
        9.153035316963035,
        9.86048622524424,
        8.3974499571329,
        7.8338007571397865,
        8.78260490259886,
        6.165829618300345,
        7.134819943097564,
        7.803701771392344,
        5.056638985134191,
        7.424601077645588,
        8.538658023474753,
        2.4616388562274896,
        0.5444633747829343,
        9.005740508584667,
    ]);
    siblings(&mut c);
    assert_eq!(intersects_any(&c), false);

    let mut c = radii(&[
        2.23606797749979,
        52.07088264296293,
        5.196152422706632,
        20.09975124224178,
        357.11557267679996,
        4.898979485566356,
        14.7648230602334,
        17.334875731491763,
    ]);
    siblings(&mut c);
    assert_eq!(intersects_any(&c), false);
}

#[test]
fn pack_siblings_circles_can_successfully_pack_a_circle_with_a_tiny_radius() {
    let mut c = radii(&[
        0.5672035864083508,
        0.6363498687452267,
        0.5628456216244132,
        1.5619458670239148,
        1.5658933259424268,
        0.9195955097595698,
        0.4747083763630309,
        0.38341282734497434,
        1.3475593361729394,
        0.7492342961633259,
        1.0716990115071823,
        0.31686823341701664,
        2.8766442376551415e-7,
    ]);
    siblings(&mut c);
    assert_eq!(intersects_any(&c), false);
}

#[test]
fn pack_siblings_accepts_large_circles() {
    let mut c = radii(&[1e11, 1.0, 1.0]);
    siblings(&mut c);
    assert_eq!(
        c,
        [
            Circle { r: 1e11, x: 0.0, y: 0.0 },
            Circle { r: 1.0, x: 1e11 + 1.0, y: 0.0 },
            Circle { r: 1.0, x: 1e11 + 1.0, y: 2.0 },
        ]
    );

    let mut c = radii(&[1e16, 1.0, 1.0]);
    siblings(&mut c);
    assert_eq!(
        c,
        [
            Circle { r: 1e16, x: 0.0, y: 0.0 },
            Circle { r: 1.0, x: 1e16 + 1.0, y: 0.0 },
            Circle { r: 1.0, x: 1e16 + 1.0, y: 2.0 },
        ]
    );
}

// ----------------------------------------------------- deterministic-test.js

/// `stratify().path(d => d)` over `/${i}/${i}-${j}` paths: a root with one
/// child per `i`, each holding `n` leaves, in input order. Built directly,
/// since `src/stratify.rs` is not ported yet.
fn deterministic_data() -> Datum {
    let mut children = Vec::new();
    for (i, n) in [41, 41, 11, 11, 4, 4].iter().enumerate() {
        let leaves: Vec<Datum> = (0..*n).map(|j| json!({"id": format!("/{}/{}-{}", i, i, j)})).collect();
        children.push(json!({"id": format!("/{}", i), "children": leaves}));
    }
    json!({"id": "/", "children": children})
}

fn xyr(t: &Tree) -> Vec<(f64, f64, f64)> {
    t.order_before(t.root)
        .into_iter()
        .map(|i| (t.nodes[i].x, t.nodes[i].y, t.nodes[i].r))
        .collect()
}

#[test]
fn pack_is_deterministic() {
    let data = deterministic_data();
    let packer = Pack::new().size([100.0, 100.0]).padding_constant(0.0);
    let mut root = hierarchy(&data);
    root.count();
    packer.pack(&mut root);
    let pack1 = xyr(&root);
    for _ in 0..40 {
        let mut root = hierarchy(&data);
        root.count();
        packer.pack(&mut root);
        assert_eq!(xyr(&root), pack1);
    }
}

// ------------------------------------------------------------- flare-test.js

fn flare_csv() -> std::path::PathBuf {
    common::data("flare.csv")
}
fn flare_pack_json() -> std::path::PathBuf {
    common::data("flare-pack.json")
}

/// `stratify().parentId(…)` over flare.csv: the parent id is the id up to the
/// last ".". Built directly, since `src/stratify.rs` is not ported yet; the
/// children of each node stay in csv row order, as stratify leaves them.
fn flare() -> Datum {
    let text = std::fs::read_to_string(flare_csv()).unwrap();
    let mut lines = text.lines();
    lines.next(); // header: id,value
    let mut ids: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();
    let mut children: Vec<Vec<usize>> = Vec::new();
    let mut parent: Vec<Option<usize>> = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (id, value) = line.split_once(',').unwrap();
        let p = match id.rfind('.') {
            Some(i) => {
                let pid = &id[..i];
                Some(ids.iter().position(|x| x == pid).unwrap())
            }
            None => None,
        };
        ids.push(id.to_string());
        values.push(value.to_string());
        children.push(Vec::new());
        parent.push(p);
        let i = ids.len() - 1;
        if let Some(p) = p {
            children[p].push(i);
        }
    }

    fn build(i: usize, ids: &[String], values: &[String], children: &[Vec<usize>]) -> Datum {
        let mut d = json!({"id": ids[i], "value": values[i]});
        if !children[i].is_empty() {
            let c: Vec<Datum> = children[i]
                .iter()
                .map(|&k| build(k, ids, values, children))
                .collect();
            d["children"] = Datum::Array(c);
        }
        d
    }

    let root = parent.iter().position(|p| p.is_none()).unwrap();
    build(root, &ids, &values, &children)
}

/// `Math.round(x * 100) / 100`.
fn round(x: f64) -> f64 {
    (x * 100.0 + 0.5).floor() / 100.0
}

#[test]
fn pack_flare_produces_the_expected_result() {
    let data = flare();
    let expected: Datum =
        serde_json::from_str(&std::fs::read_to_string(flare_pack_json()).unwrap()).unwrap();

    let mut root = hierarchy(&data);
    // .sum(d => d.value): the csv values are strings, coerced with `+v || 0`.
    root.sum(|d| {
        let s = d["value"].as_str().unwrap_or("");
        if s.is_empty() {
            0.0
        } else {
            s.parse::<f64>().unwrap_or(f64::NAN)
        }
    });
    // .sort((a, b) => b.value - a.value || a.data.id.localeCompare(b.data.id))
    root.sort(|a, b| {
        let d = b.value.unwrap() - a.value.unwrap();
        if d < 0.0 {
            std::cmp::Ordering::Less
        } else if d > 0.0 {
            std::cmp::Ordering::Greater
        } else {
            a.data["id"].as_str().unwrap().cmp(b.data["id"].as_str().unwrap())
        }
    });

    Pack::new().size([960.0, 960.0]).pack(&mut root);

    // The JS compares value, x, y, r, name and children (id, parent, data,
    // depth and height are deleted before the comparison).
    fn visit(t: &Tree, i: usize, expected: &Datum, path: &str) {
        let n = &t.nodes[i];
        let id = n.data["id"].as_str().unwrap();
        let name = &id[id.rfind('.').map(|k| k + 1).unwrap_or(0)..];
        assert_eq!(expected["name"].as_str().unwrap(), name, "name at {}", path);
        assert_eq!(expected["value"].as_f64().unwrap(), n.value.unwrap(), "value at {}", path);
        assert_eq!(expected["x"].as_f64().unwrap(), round(n.x), "x at {}", path);
        assert_eq!(expected["y"].as_f64().unwrap(), round(n.y), "y at {}", path);
        assert_eq!(expected["r"].as_f64().unwrap(), round(n.r), "r at {}", path);
        match (&n.children, expected.get("children")) {
            (Some(c), Some(e)) => {
                let e = e.as_array().unwrap();
                assert_eq!(c.len(), e.len(), "children length at {}", path);
                for (k, &child) in c.iter().enumerate() {
                    visit(t, child, &e[k], &format!("{}/{}", path, name));
                }
            }
            (None, None) => {}
            _ => panic!("children mismatch at {}/{}", path, name),
        }
    }
    visit(&root, root.root, &expected, "");
}
