// Microbenchmark de pesquisa, 2026-10-02; nao e um teste do renderer GPUI.
// tick e Node::{radius,charge} extraidos sem alterar calculos de
// apps/desktop-gpui/src/screens/graph.rs, base 7cb779a9754ae2ff51d6c629b6c14f3e38c7440c.
// Node reduzido aos campos usados; arestas Affects com rest=58; ilhas de 25.
// Memoria, seed, build/place, paint, GPU e consultas reais nao sao medidos.
// rustc -C opt-level=1 --edition=2021 tick-benchmark.rs -o <diretorio-temporario>/tick-benchmark.exe
// Executar o binario fora do repositorio; 3 repeticoes por tamanho, checksum consumido.
#![allow(dead_code)]
use std::time::Instant;
#[derive(Clone, Copy)]
enum Kind {
    Component,
    Technology,
    Decision,
    Rule,
}
struct Node {
    kind: Kind,
    cluster: Option<usize>,
    degree: usize,
    pos: (f32, f32),
    vel: (f32, f32),
}
struct Link {
    a: usize,
    b: usize,
}
impl Link {
    fn rest(&self) -> f32 {
        58.0
    }
}
impl Node {
    fn radius(&self) -> f32 {
        match self.kind {
            Kind::Component => 11.0 + (self.degree.min(12) as f32) * 1.3,
            Kind::Technology => 8.5,
            Kind::Decision => 3.6,
            Kind::Rule => 4.2,
        }
    }

    fn charge(&self) -> f32 {
        match self.kind {
            Kind::Component => 5200.0,
            Kind::Technology => 1600.0,
            Kind::Decision | Kind::Rule => 520.0,
        }
    }
}

fn tick(nodes: &mut [Node], links: &[Link], heat: f32) {
    let count = nodes.len();
    for i in 0..count {
        for j in (i + 1)..count {
            let (dx, dy) = (
                nodes[j].pos.0 - nodes[i].pos.0,
                nodes[j].pos.1 - nodes[i].pos.1,
            );
            let mut d2 = dx * dx + dy * dy;
            if d2 < 0.01 {
                d2 = 0.01;
            }
            let distance = d2.sqrt();
            let strength = (nodes[i].charge() + nodes[j].charge()) * 0.5 / d2 * heat;
            let (ux, uy) = (dx / distance, dy / distance);
            nodes[i].vel.0 -= ux * strength;
            nodes[i].vel.1 -= uy * strength;
            nodes[j].vel.0 += ux * strength;
            nodes[j].vel.1 += uy * strength;
            // Collision keeps shapes and labels from piling up.
            let room = nodes[i].radius() + nodes[j].radius() + 14.0;
            if distance < room {
                let push = (room - distance) * 0.25;
                nodes[i].pos.0 -= ux * push;
                nodes[i].pos.1 -= uy * push;
                nodes[j].pos.0 += ux * push;
                nodes[j].pos.1 += uy * push;
            }
        }
    }
    for link in links {
        let (a, b) = (link.a, link.b);
        let (dx, dy) = (
            nodes[b].pos.0 - nodes[a].pos.0,
            nodes[b].pos.1 - nodes[a].pos.1,
        );
        let distance = (dx * dx + dy * dy).sqrt().max(0.1);
        let pull = (distance - link.rest()) * 0.06 * heat;
        let (ux, uy) = (dx / distance, dy / distance);
        nodes[a].vel.0 += ux * pull;
        nodes[a].vel.1 += uy * pull;
        nodes[b].vel.0 -= ux * pull;
        nodes[b].vel.1 -= uy * pull;
    }
    // Islands keep apart: tops repel within a wide reach.
    let tops: Vec<usize> = (0..count)
        .filter(|index| nodes[*index].cluster == Some(*index))
        .collect();
    for (order, &i) in tops.iter().enumerate() {
        for &j in tops.iter().skip(order + 1) {
            let (dx, dy) = (
                nodes[j].pos.0 - nodes[i].pos.0,
                nodes[j].pos.1 - nodes[i].pos.1,
            );
            let distance = (dx * dx + dy * dy).sqrt().max(0.1);
            let reach = 230.0;
            if distance < reach {
                let push = (reach - distance) * 0.05 * heat.max(0.2);
                let (ux, uy) = (dx / distance, dy / distance);
                nodes[i].vel.0 -= ux * push;
                nodes[i].vel.1 -= uy * push;
                nodes[j].vel.0 += ux * push;
                nodes[j].vel.1 += uy * push;
            }
        }
    }
    let anchors: Vec<Option<(f32, f32)>> = nodes
        .iter()
        .map(|node| node.cluster.map(|cluster| nodes[cluster].pos))
        .collect();
    for (node, anchor) in nodes.iter_mut().zip(anchors) {
        if let Some(anchor) = anchor {
            node.vel.0 += (anchor.0 - node.pos.0) * 0.012 * heat;
            node.vel.1 += (anchor.1 - node.pos.1) * 0.012 * heat;
        }
        // Gravity keeps islands and loose technologies near the middle, so
        // the map reads as one picture instead of scattered parts.
        node.vel.0 -= node.pos.0 * 0.011 * heat;
        node.vel.1 -= node.pos.1 * 0.011 * heat;
        node.pos.0 += node.vel.0;
        node.pos.1 += node.vel.1;
        node.vel.0 *= 0.55;
        node.vel.1 *= 0.55;
    }
}

/// Entrance order: islands first, from the center out.

fn main() {
    println!("nodes,edges,steps,run,elapsed_ms,checksum");
    for n in [100usize, 500, 1000, 2500] {
        for run in 0..3 {
            let mut nodes: Vec<Node> = (0..n)
                .map(|i| Node {
                    kind: if i % 25 == 0 {
                        Kind::Component
                    } else {
                        Kind::Decision
                    },
                    cluster: Some(i / 25 * 25),
                    degree: 1,
                    pos: ((i % 50) as f32 * 23.0, (i / 50) as f32 * 23.0),
                    vel: (0.0, 0.0),
                })
                .collect();
            let links: Vec<Link> = (0..n)
                .filter(|i| i % 25 != 0)
                .map(|i| Link {
                    a: i / 25 * 25,
                    b: i,
                })
                .collect();
            let start = Instant::now();
            let mut heat = 1.0f32;
            for _ in 0..420 {
                tick(&mut nodes, &links, heat);
                heat = (heat * 0.988).max(0.02);
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            let sum: f32 = nodes.iter().map(|node| node.pos.0 + node.pos.1).sum();
            println!(
                "{n},{},420,{run},{ms:.3},{}",
                links.len(),
                std::hint::black_box(sum)
            );
        }
    }
}
