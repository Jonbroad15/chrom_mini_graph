use crate::align;
use crate::avl_tree::SearchTree;
use crate::data_structs::KmerNode;
use crate::data_structs::{Anchors, Color};
use debruijn::kmer::Kmer16;
use debruijn::Kmer;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use smallvec::SmallVec;
use std::mem;
use std::time::Instant;

pub fn print_as_binary(color: Color, string: String) {
    let binary_color = format!("{:#08b}", color);
    println!("{},{}", string, binary_color);
}

//                    let parent_color= format!("{:#08b}", parent_path_color);

pub fn get_kmer_dict_mut(seeds: &mut Vec<KmerNode>) -> FxHashMap<Kmer16, Vec<u32>> {
    let mut mini_hash_map = FxHashMap::default();
    for (_i, kmer_node) in seeds.iter().enumerate() {
        let kmer = &kmer_node.kmer;
        let pos_vec = mini_hash_map.entry(*kmer).or_insert(vec![]);
        pos_vec.push(kmer_node.id);
    }

    return mini_hash_map;
}

pub fn get_kmer_dict(seeds: &Vec<KmerNode>) -> FxHashMap<Kmer16, Vec<u32>> {
    let mut mini_hash_map = FxHashMap::default();
    for (_i, kmer_node) in seeds.iter().enumerate() {
        let kmer = &kmer_node.kmer;
        let pos_vec = mini_hash_map.entry(*kmer).or_insert(vec![]);
        pos_vec.push(kmer_node.id);
    }

    return mini_hash_map;
}

pub fn position_max_f64(slice: &[f64]) -> Option<usize> {
    slice
        .iter()
        .enumerate()
        .max_by(|(_, value0), (_, value1)| value0.partial_cmp(value1).unwrap())
        .map(|(idx, _)| idx)
}

pub fn position_max_f64_nth(slice: &[f64], n: usize) -> usize {
    let mut vec: Vec<_> = slice.iter().enumerate().collect();
    vec.sort_by(|(_, v0), (_, v1)| v1.partial_cmp(v0).unwrap());
    return vec[n].0;
}

#[inline]
fn min_dist_ref_nodes(ref_node1: &Vec<(usize, usize)>, ref_node2: &Vec<(usize, usize)>) -> f64 {
    let mut used_colors = vec![];
    let mut color_map = vec![usize::MAX; 128];
    for (col, pos) in ref_node2 {
        used_colors.push(col);
        color_map[*col] = *pos;
    }
    for (col, pos) in ref_node1 {
        if color_map[*col] < *pos {
            color_map[*col] = *pos - color_map[*col];
        } else {
            color_map[*col] -= *pos;
        }
    }

    let mut best_dist = usize::MAX;
    for col in used_colors {
        if color_map[*col] < best_dist {
            best_dist = color_map[*col];
        }
    }
    return best_dist as f64;
}

#[inline]
fn min_dist_ref_nodes_lazy(
    ref_node1: &Vec<(usize, usize)>,
    ref_node2: &Vec<(usize, usize)>,
    shared_col: Color,
) -> f64 {
    let shared_col_ind = align::get_first_nonzero_bit(shared_col);
    let mut shared_pos = 0;
    for (col, pos) in ref_node2 {
        if *col == shared_col_ind {
            shared_pos = *pos;
            break;
        }
    }
    for (col, pos) in ref_node1 {
        if *col == shared_col_ind {
            if shared_pos < *pos {
                return (*pos - shared_pos) as f64;
            } else {
                return (shared_pos - *pos) as f64;
            }
        }
    }

    return f64::MAX;
}

#[inline]
fn heuristic_super_score(ref_order_dist: f64, query_order_dist: f64, samp_freq: usize) -> f64 {
    let gap = f64::abs(ref_order_dist - query_order_dist);
    let gap_cost;
    let dist_between_nodes_guess = 200;
    if gap < (samp_freq * dist_between_nodes_guess) as f64 {
        gap_cost = gap * 2.;
    } else {
        gap_cost = gap * 5.;
        //        gap_cost = (gap - (samp_freq * dist_between_nodes_guess) as f64) / samp_freq as f64;
    }
    let score = dist_between_nodes_guess as f64 - gap_cost;

    return score;
}

#[inline]
fn heuristic_score(ref_order_dist: f64, query_order_dist: f64) -> f64 {
    let dist_ref = -2.0 * f64::sqrt(ref_order_dist);
    //    let dist_ref = -1.0 * ref_order_dist;
    let dist_query = 30.0 - 2. * query_order_dist;
    //    let gap_cost = -f64::max((ref_order_dist - query_order_dist).abs(), 10.);
    let score = dist_ref + dist_query;

    return score;
}

fn score(ref_order_dist: f64, query_order_dist: f64) -> f64 {
    let dist_ref = 100.0 - 1.0 * ref_order_dist;
    let dist_query = 100.0 - 3.0 * query_order_dist;
    let score = dist_ref + dist_query;
    //try max chain length score

    //    if score < 0.0{
    //        score = 0.05 * score - score.abs().log2();
    //    }
    //    let num_bases_query = f64::max(dist_query, 0.0);
    //    let num_bases_ref = f64::max(dist_ref, 0.0);
    //    let mut overlap = f64::min(num_bases_query, num_bases_ref);

    //    let score;
    //    if overlap < 0.001 {
    //        overlap = f64::max(dist_ref, dist_query);
    //        //        score = (f64::min(dist_query, dist_ref) / 10.0).exp();
    //        //        score = f64::min(dist_query, dist_ref).exp();
    //        score = 0.0;
    //    } else {
    //        score = overlap;
    //    }
    //
    return score;
}

fn _alpha(j: usize, i: usize, anchors: &Vec<(&KmerNode, &KmerNode)>) -> f64 {
    let dist_multiplier = 1.0;
    let num_bases_query = f64::max(
        16.0 - dist_multiplier * (anchors[i].0.order as f64 - anchors[j].0.order as f64),
        0.0,
    );
    let num_bases_ref = f64::max(
        16.0 - dist_multiplier * (anchors[i].1.order as f64 - anchors[j].1.order as f64),
        0.0,
    );
    return f64::min(num_bases_query, num_bases_ref);
}

fn beta(ref_order_dist: f64, query_order_dist: f64) -> f64 {
    let dist_multiplier = 5.0;

    //    if f64::max(ref_diff, query_diff) > g {
    //        return f64::MAX;
    //    }

    let diff = (query_order_dist - ref_order_dist) * dist_multiplier;
    if diff as usize > 1000000 {
        return f64::MAX;
    }
    if diff == 0.0 {
        return 0.0;
    } else {
        //        return (0.01 * 16.0 as f64 * diff) + 0.5 * diff.log2();
        //        return diff.log2()/100.0;
        return 0.0;
    }
}

pub fn get_super_chains(
    seeds_ref: &Vec<KmerNode>,
    seeds_q: &Vec<KmerNode>,
    ref_hash_map: &FxHashMap<Kmer16, Vec<u32>>,
    q_hash_map: &FxHashMap<Kmer16, Vec<u32>>,
    h: usize,
    chain_heuristic: bool,
    not_used_kmers: &FxHashSet<Kmer16>,
    closest_kmer_vec: &Vec<Option<u32>>,
    samp_freq: usize,
    read_length: usize,
) -> Vec<(Anchors, f64, bool)> {
    let now = Instant::now();
    let (forward_anchors, backward_anchors, num_forward_anchors, num_backward_anchors) =
        anchors_from_seeds(seeds_ref, seeds_q, ref_hash_map, q_hash_map, not_used_kmers);

    let forward_strand;
    if num_forward_anchors > num_backward_anchors {
        forward_strand = true;
    } else if num_backward_anchors > num_forward_anchors {
        forward_strand = false;
    } else {
        forward_strand = true;
    }

    let normal_anchors;
    if forward_strand {
        normal_anchors = forward_anchors;
    } else {
        normal_anchors = backward_anchors;
    }

    let mut super_nodes_map = FxHashMap::default();
    for (ref_id, q_id) in normal_anchors.iter() {
        let opt = closest_kmer_vec[(*ref_id) as usize];
        let closest_coord_node;
        if !opt.is_none() {
            closest_coord_node = opt.unwrap();
        } else {
            continue;
        }
        //TODO can RC the vector inside the container for performance bonus
        let list_of_hits = super_nodes_map.entry(closest_coord_node).or_insert(vec![]);
        list_of_hits.push((seeds_q[(*q_id) as usize].actual_ref_positions[0], *q_id));
    }

    let mut debug = vec![];
    let mut all_colors = FxHashSet::default();
    for (coord_node, list_of_hits) in super_nodes_map.iter_mut() {
        let cutoff = (0.35 * 0.9f64.powf(16.) * samp_freq as f64) as usize;
        if list_of_hits.len() > cutoff {
            let ref_pos = &seeds_ref[(*coord_node) as usize].actual_ref_positions;
            let order = &seeds_ref[(*coord_node) as usize].order;
            let color = seeds_ref[(*coord_node) as usize].color;
            let list_color = align::get_nonzero_bits(color);
            let list_color: Vec<usize> = list_color.into_iter().rev().collect();
            for color_ind in list_color.iter() {
                all_colors.insert(*color_ind);
            }
            let col_ref_zip: Vec<(usize, &usize)> =
                list_color.into_iter().zip(ref_pos.into_iter()).collect();
            debug.push((order, col_ref_zip, std::mem::take(list_of_hits), coord_node))
        }
    }
    debug.sort_by(|x, y| x.0.cmp(&y.0));

    //    dbg!(&debug);
    let mut super_anchors = vec![];
    let mut col_list = FxHashMap::default();
    for (_i, (_order, col_pos, hits, coord_node)) in debug.iter().enumerate() {
        for (_pos, q_id) in hits {
            super_anchors.push((**coord_node, *q_id));
        }
        let mut list = vec![];
        for (col, pos) in col_pos {
            list.push((*col, **pos));
        }
        col_list.insert(*coord_node, list);
    }

    println!(
        "Super chain preprocess time {}",
        now.elapsed().as_secs_f32()
    );
    let now = Instant::now();

    super_anchors.sort_by(|x, y| x.0.cmp(&y.0));
    let mut ref_color_vec = vec![0; super_anchors.len()];
    for (i, anchor) in super_anchors.iter().enumerate() {
        ref_color_vec[i] = seeds_ref[(anchor.0) as usize].color;
    }
    let mut pointer_array = vec![0; super_anchors.len()];
    let mut f = vec![0.];
    for i in 1..super_anchors.len() {
        let mut best_f_i = 0. as f64;
        let mut best_j = usize::MAX;
        let mut start = 0;
        if i > h {
            start = i - h
        }
        for j in start..i {
            let anchorjq;
            let anchoriq;
            if forward_strand {
                anchorjq = seeds_q[super_anchors[j].1 as usize].actual_ref_positions[0];
                anchoriq = seeds_q[super_anchors[i].1 as usize].actual_ref_positions[0];
            } else {
                anchorjq =
                    read_length - seeds_q[super_anchors[j].1 as usize].actual_ref_positions[0];
                anchoriq =
                    read_length - seeds_q[super_anchors[i].1 as usize].actual_ref_positions[0];
            }

            let anchorjr_o = seeds_ref[super_anchors[j].0 as usize].order;
            let anchorir_o = seeds_ref[super_anchors[i].0 as usize].order;

            let anchorjr_n = &seeds_ref[super_anchors[j].0 as usize];
            let anchorir_n = &seeds_ref[super_anchors[i].0 as usize];

            let color_history_jr = ref_color_vec[j];
            let color_ir = seeds_ref[super_anchors[i].0 as usize].color;

            let mut incompat_chain = false;

            if anchorjr_o >= anchorir_o {
                incompat_chain = true;
            }

            //This forces the chain to be a walkable path in the DAG
            if color_history_jr & color_ir == 0 {
                incompat_chain = true;
            }

            if anchorjq >= anchoriq {
                incompat_chain = true;
            }

            let f_cand_i;
            let use_lazy = false;

            if incompat_chain {
                f_cand_i = f64::MIN;
            } else {
                //                let ref_order_dist = min_dist_ref_nodes(
                //                    &col_list[&super_anchors[j].0],
                //                    &col_list[&super_anchors[i].0],
                //                );
                let ref_order_dist;
                if use_lazy {
                    ref_order_dist = min_dist_ref_nodes_lazy(
                        &col_list[&super_anchors[j].0],
                        &col_list[&super_anchors[i].0],
                        color_history_jr & color_ir,
                    );
                } else {
                    ref_order_dist = min_dist_ref_nodes(
                        &col_list[&super_anchors[j].0],
                        &col_list[&super_anchors[i].0],
                    );
                }

                let query_order_dist = (anchoriq - anchorjq) as f64;
                f_cand_i = f[j] + heuristic_super_score(ref_order_dist, query_order_dist, samp_freq)
            }
            if f_cand_i > best_f_i {
                best_f_i = f_cand_i;
                best_j = j;
            }
        }
        if best_f_i <= 0.0 {
            best_j = i
        }
        f.push(best_f_i);
        if best_j != usize::MAX {
            pointer_array[i] = best_j;
        }
    }

    let mut best = get_best_chains(f, pointer_array, &super_anchors, seeds_ref, seeds_q, true);

    println!("Super chain chaining time {}", now.elapsed().as_secs_f32());
    if best.is_empty() {
        println!("no best superchain");
        return vec![];
    } else {
        println!(
            "{:?}\n{:?}",
            seeds_ref[best[0].0[0].0 as usize],
            seeds_ref[best[0].0.last().unwrap().0 as usize]
        );
        println!(
            "{:?}\n{:?}",
            seeds_q[best[0].0[0].1 as usize],
            seeds_q[best[0].0.last().unwrap().1 as usize]
        );

        println!("Number of chains:{}", best.len());
        for i in 0..best.len() {
            for anchor in best[i].0.iter() {
                //                println!("q-{:?}",seeds_q[anchor.1 as usize]);
                //                println!("r-{:?}",seeds_ref[anchor.0 as usize]);
            }
        }
        return best
            .iter_mut()
            .map(|x| (mem::take(&mut x.0), x.3, forward_strand))
            .collect();
    }

    //    let do_interval_stuff = false;
    //    if do_interval_stuff {
    //        let mut interval_scores = vec![];
    //        let mut closed_interval = (0, 0);
    //        let mut running_diff = 0;
    //        for i in 0..pos_mat.len() - 1 {
    //            let v2 = &pos_mat[i + 1];
    //            let mut min_diff = i64::MAX;
    //            for k in closed_interval.0..closed_interval.1 + 1 {
    //                let v1 = &pos_mat[k];
    //                for j in 0..128 {
    //                    if v1[j] == -1 || v2[j] == -1 {
    //                        continue;
    //                    }
    //                    let diff = v2[j] - v1[j];
    //                    if diff < min_diff {
    //                        min_diff = diff;
    //                    }
    //                }
    //            }
    //
    //            if min_diff == i64::MAX {
    //                min_diff = 0;
    //            }
    //            running_diff += min_diff;
    //            if running_diff as f64 > read_length as f64 * 2.25 {
    //                let mut interval_set = FxHashSet::default();
    //                for j in closed_interval.0..closed_interval.1 + 1 {
    //                    let hits = &debug[j].2;
    //                    for hit in hits.iter() {
    //                        interval_set.insert(hit);
    //                    }
    //                }
    //                interval_scores.push((closed_interval, interval_set.len()));
    //                closed_interval = (i + 1, i + 1);
    //                running_diff = 0;
    //            } else {
    //                closed_interval.1 += 1;
    //            }
    //        }
    //
    //        interval_scores.sort_by(|x, y| y.1.cmp(&x.1));
    //        for k in 0..2 {
    //            dbg!(
    //                "------------------------------------ {} ",
    //                interval_scores[k].1
    //            );
    //            for i in (interval_scores[k].0 .0)..(interval_scores[k].0 .1 + 1) {
    //                dbg!(&debug[i]);
    //            }
    //        }
    //    }
}

fn get_chains<'a>(
    seeds_ref: &'a Vec<KmerNode>,
    seeds_q: &'a mut Vec<KmerNode>,
    h: usize,
    chain_heuristic: bool,
    chain_reads: bool,
    circular: bool,
    forward_strand: bool,
    anchors: &mut Vec<(u32, u32)>,
) -> Vec<(Anchors, f64)> {
    let q_len = seeds_q.len();
    if forward_strand == false {
        for node in seeds_q.iter_mut() {
            node.order = q_len as u32 - node.order - 1;
            //            for child_id in node.child_nodes.iter_mut(){
            //                *child_id = q_len as u32 - *child_id - 1;
            //            }
        }
    }

    if anchors.len() == 0 {
        return vec![(vec![], 0.0)];
    }

    //dbg!(anchors[1],anchors[2],anchors[3]);
    //dbg!(alpha(1,2,&anchors,k),alpha(2,3,&anchors,k));
    //dbg!(beta(1,2,&anchors,k,g),beta(2,3,&anchors,k,g));
    //chaining

    let mut f = vec![0.0 as f64];
    f.reserve(anchors.len());
    let mut pointer_array = vec![];
    for i in 0..anchors.len() {
        pointer_array.push(i);
    }

    anchors.sort_by(|a, b| {
        seeds_ref[a.0 as usize]
            .order
            .cmp(&seeds_ref[b.0 as usize].order)
    });

    score_anchors(
        &mut f,
        &mut pointer_array,
        anchors,
        chain_heuristic,
        chain_reads,
        h,
        &seeds_ref,
        &seeds_q,
        (0, 0),
    );

    //    let (mut best_seq_anchors_1, range_ref, range_query) =
    let mut chain_range_vec = get_best_chains(
        f,
        pointer_array,
        &anchors,
        &seeds_ref,
        &seeds_q,
        chain_reads,
    );

    //Return all secondary chains for read chaining
    if chain_reads {
        return chain_range_vec
            .iter_mut()
            .map(|x| (mem::take(&mut x.0), x.3))
            .collect();
    }
    //Return best chain for genome chaining
    else if !circular {
        return vec![(mem::take(&mut chain_range_vec[0].0), chain_range_vec[0].3)];
    } else {
        //For second round of chaining, include anchors outside the range
        //of previous chain
        let mut best_seq_anchors_1 = mem::take(&mut chain_range_vec[0].0);
        let range_ref = chain_range_vec[0].1;
        let range_query = chain_range_vec[0].2;
        let best_aln_score = chain_range_vec[0].3;
        let mut second_round_anchors = vec![];
        for anchor in anchors {
            if seeds_ref[anchor.0 as usize].order < range_ref.0
                || seeds_ref[anchor.0 as usize].order > range_ref.1
            {
                if seeds_q[anchor.1 as usize].order < range_query.0
                    || seeds_q[anchor.1 as usize].order > range_query.1
                {
                    second_round_anchors.push(*anchor);
                }
            }
        }

        let mut f = vec![0.0 as f64];
        let mut pointer_array = vec![];
        for i in 0..second_round_anchors.len() {
            pointer_array.push(i);
        }

        if second_round_anchors.len() == 0 || chain_reads || !circular {
            return vec![(best_seq_anchors_1, best_aln_score)];
        }

        score_anchors(
            &mut f,
            &mut pointer_array,
            &mut second_round_anchors,
            chain_heuristic,
            chain_reads,
            h,
            &seeds_ref,
            &seeds_q,
            (range_ref.1, range_query.1),
        );

        let (mut best_seq_anchors_2, _range_ref, _range_query, second_best_aln_score) = mem::take(
            &mut get_best_chains(
                f,
                pointer_array,
                &second_round_anchors,
                &seeds_ref,
                &seeds_q,
                chain_reads,
            )[0],
        );

        best_seq_anchors_1.append(&mut best_seq_anchors_2);
        //Reverse back because we may not want q reversed if the best alignment was actually forward.
        if forward_strand == false {
            for node in seeds_q.iter_mut() {
                node.order = q_len as u32 - node.order - 1;
                //            for child_id in node.child_nodes.iter_mut(){
                //                *child_id = q_len as u32 - *child_id - 1;
                //            }
            }
        }
        //    best_seq_anchors_1.sort_by(|a, b| a.0.cmp(&b.0));
        return vec![(best_seq_anchors_1, second_best_aln_score + best_aln_score)];
    }
}

fn anchors_from_seeds(
    seeds_ref: &Vec<KmerNode>,
    seeds_q: &Vec<KmerNode>,
    ref_hash_map: &FxHashMap<Kmer16, Vec<u32>>,
    q_hash_map: &FxHashMap<Kmer16, Vec<u32>>,
    not_used_kmers: &FxHashSet<Kmer16>,
) -> (Anchors, Anchors, usize, usize) {
    let mut forward_anchors = vec![];
    let mut forward_hits_set = FxHashSet::default();
    let mut backward_anchors = vec![];
    let mut backward_hits_set = FxHashSet::default();

    for kmer in q_hash_map.keys() {
        if not_used_kmers.contains(kmer) {
            continue;
        }
        if ref_hash_map.contains_key(kmer) {
            let mut count = 0;
            let ref_positions = ref_hash_map.get(kmer).unwrap();
            let query_positions = q_hash_map.get(kmer).unwrap();
            for p1 in ref_positions {
                for p2 in query_positions {
                    let node_r = &seeds_ref[*p1 as usize];
                    let node_q = &seeds_q[*p2 as usize];
                    if node_r.canonical == node_q.canonical {
                        //                        forward_anchors.push((node_r,node_q));
                        forward_anchors.push((node_r.id, node_q.id));
                        forward_hits_set.insert(&node_r.kmer);
                    } else {
                        //                        backward_anchors.push((node_r,node_q));
                        backward_anchors.push((node_r.id, node_q.id));
                        backward_hits_set.insert(&node_r.kmer);
                    }
                    count += 1
                }
            }
        }
    }

    //    let num_forward_anchors = forward_anchors.len();
    //    let num_backward_anchors = backward_anchors.len();
    let num_forward_anchors = forward_hits_set.len();
    let num_backward_anchors = backward_hits_set.len();

    return (
        forward_anchors,
        backward_anchors,
        num_forward_anchors,
        num_backward_anchors,
    );
}

pub fn chain_seeds<'a>(
    seeds_ref: &'a Vec<KmerNode>,
    seeds_q: &'a mut Vec<KmerNode>,
    ref_hash_map: &'a FxHashMap<Kmer16, Vec<u32>>,
    q_hash_map: &'a FxHashMap<Kmer16, Vec<u32>>,
    h: usize,
    chain_heuristic: bool,
    chain_reads: bool,
    not_used_kmers: &FxHashSet<Kmer16>,
    circular: bool,
) -> Vec<(Anchors, f64, bool)> {
    let q_len = seeds_q.len();
    let now = Instant::now();

    let (mut forward_anchors, mut backward_anchors, num_forward_anchors, num_backward_anchors) =
        anchors_from_seeds(seeds_ref, seeds_q, ref_hash_map, q_hash_map, not_used_kmers);

    let mut forward_strand;
    let ambig;
    if num_forward_anchors > num_backward_anchors * 10 / 8 {
        forward_strand = true;
        ambig = false;
    } else if num_backward_anchors > num_forward_anchors * 10 / 8 {
        forward_strand = false;
        ambig = false;
    } else {
        ambig = true;
        forward_strand = true;
    }

    println!(
        "Num forward/back anchors {},{}, Query length {}",
        forward_anchors.len(),
        backward_anchors.len(),
        seeds_q.len()
    );

    if !ambig && !chain_reads {
        let mut anchors;
        if forward_strand {
            anchors = forward_anchors;
        } else {
            anchors = backward_anchors
        }
        let mut chains_scores = get_chains(
            seeds_ref,
            seeds_q,
            h,
            chain_heuristic,
            chain_reads,
            circular,
            forward_strand,
            &mut anchors,
        );
        return chains_scores
            .iter_mut()
            .map(|x| (mem::take(&mut x.0), x.1, forward_strand))
            .collect();
        //vec![(best_chain, aln_score, forward_strand)];
    } else {
        forward_strand = true;
        let chains_scores_forward = get_chains(
            seeds_ref,
            seeds_q,
            h,
            chain_heuristic,
            chain_reads,
            circular,
            forward_strand,
            &mut forward_anchors,
        );
        forward_strand = false;
        let chains_scores_backward = get_chains(
            seeds_ref,
            seeds_q,
            h,
            chain_heuristic,
            chain_reads,
            circular,
            forward_strand,
            &mut backward_anchors,
        );
        let mut return_chains = vec![];
        for (chain, score) in chains_scores_forward.into_iter() {
            return_chains.push((chain, score, true));
        }
        for (chain, score) in chains_scores_backward.into_iter() {
            return_chains.push((chain, score, false));
        }
        return return_chains;

        //        if aln_score_backward > aln_score_forward {
        //            for node in seeds_q.iter_mut() {
        //                node.order = q_len as u32 - node.order - 1;
        //                //            for child_id in node.child_nodes.iter_mut(){
        //                //                *child_id = q_len as u32 - *child_id - 1;
        //                //            }
        //            }
        //            return (best_seq_anchors_backward, aln_score_backward, false);
        //        } else {
        //            return (best_seq_anchors_forward, aln_score_forward, true);
        //        }
    }
}

//L ← Empty list that will contain the sorted nodes
//while exists nodes without a permanent mark do
//    select an unmarked node n
//    visit(n)
//
//function visit(node n)
//    if n has a permanent mark then
//        return
//    if n has a temporary mark then
//        stop   (not a DAG)
//
//    mark n with a temporary mark
//
//    for each node m with an edge from n to m do
//        visit(m)
//
//    remove temporary mark from n
//    mark n with a permanent mark
//    add n to head of L

pub fn graph_dist(
    node_forward: &KmerNode,
    node_back: &KmerNode,
    n: usize,
    all_nodes: &Vec<KmerNode>,
) -> f64 {
    let order_dist = node_forward.order - node_back.order;
    let mut visited = FxHashSet::default();

    let mut nodes_to_visit = SmallVec::<[u32; 20]>::new();
    let mut nodes_visited = SmallVec::<[u32; 20]>::new();
    nodes_to_visit.push(node_back.id);
    for i in 0..usize::min(order_dist as usize, n) {
        for node in nodes_to_visit.iter() {
            for child in all_nodes[*node as usize].child_nodes.iter() {
                nodes_visited.push(*child);
                visited.insert(child);
            }

            if visited.contains(&node_forward.id) {
                return (i + 1) as f64;
            }
        }
        mem::swap(&mut nodes_to_visit, &mut nodes_visited);
        nodes_visited.clear();
    }

    return f64::MAX;
}

pub fn get_best_path_from_chain2(
    anchors: &Anchors,
    ref_nodes: &Vec<KmerNode>,
    order_to_id: &Vec<u32>,
    query_nodes: &Vec<KmerNode>,
) -> (Vec<Color>, Vec<(Anchors, f64)>) {
    let mut in_edges_dict: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
    let mut best_paths: FxHashMap<u32, Vec<(Color, f64, usize, usize)>> = FxHashMap::default();
    let mut current_anchor_id = 0;

    if anchors.len() == 0 {
        println!("No anchors found");
        return (vec![], vec![]);
    }

    let mut cumulative_best_score = f64::MIN;
    let last_node = &ref_nodes[anchors.last().unwrap().0 as usize];
    let first_node = &ref_nodes[anchors[0].0 as usize];
    let mut best_node = first_node;

    //Create a starting path for the first node
    for child_id in first_node.child_nodes.iter() {
        let parent_vec = in_edges_dict.entry(*child_id).or_insert(vec![]);
        parent_vec.push(first_node.id);
    }

    best_paths.insert(first_node.id, vec![(first_node.color, 10.0, 0, 0)]);
    current_anchor_id += 1;

    for i in first_node.order + 1..last_node.order + 1 {
        let id_of_node = order_to_id[i as usize];
        //        println!("Id/order of node {},{}", id_of_node, i);

        let intermediate_node = &ref_nodes[id_of_node as usize];

        for child_id in intermediate_node.child_nodes.iter() {
            let parent_vec = in_edges_dict.entry(*child_id).or_insert(vec![]);
            parent_vec.push(intermediate_node.id);
        }

        //Reevaluate paths
        let mut best_node_paths = vec![];
        best_node_paths.reserve(20);

        let parent_vec;
        //Possible path for nodes that don't have an ancestor that
        //was in anchor
        if !in_edges_dict.contains_key(&intermediate_node.id) {
            best_paths.insert(
                intermediate_node.id,
                vec![(intermediate_node.color, 0.0, 0, current_anchor_id)],
            );
            parent_vec = vec![];
        } else {
            parent_vec = in_edges_dict.get(&intermediate_node.id).unwrap().to_vec();
        }

        let anchor_hit;
        let mut unreachable_past = false;

        if anchors[current_anchor_id].0 == intermediate_node.id {
            anchor_hit = true;
        } else {
            anchor_hit = false;
            if ref_nodes[anchors[current_anchor_id].0 as usize].order < intermediate_node.order {
                unreachable_past = true;
            }
        }

        //Weird stuff happens w.r.t coloring if there is an insertion in the graph
        //Consider    o
        //          /   \
        //        o   -  o
        //
        //        if the bottom-left is 111, top is 110, bottom-right is 111, then the path
        //        from BL to BR should be 001.

        let mut all_colors = 0;
        for parent_id in parent_vec.iter() {
            //Get the color of the edge from the parent to the current node
            let parent_node = &ref_nodes[*parent_id as usize];
            let mut edge_color = 0;
            let mut bp_ref_distance = 0;
            let mut num_edges_diff_bp = 0;
            for edge in parent_node.child_edge_distance.iter() {
                let ind = edge.1 .1;
                if parent_node.child_nodes[ind as usize] == intermediate_node.id {
                    edge_color |= edge.1 .0;
                    bp_ref_distance += edge.0 as usize;
                    num_edges_diff_bp += 1;
                }
            }

            bp_ref_distance /= num_edges_diff_bp;

            let cutoff = usize::MAX;
            let parent_paths = best_paths.get_mut(parent_id).unwrap();
            if parent_paths.len() > cutoff {
                parent_paths.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
                dbg!(&parent_paths);
            }
            for l in 0..usize::min(parent_paths.len(), cutoff) {
                let parent_path = &parent_paths[l];
                let parent_path_color = parent_path.0;
                all_colors |= parent_path_color;
                //Color coherence is needed for new paths w.r.t intermediate node
                if parent_path_color & edge_color != 0 {
                    let new_color = parent_path_color & edge_color;
                    //If the intermediate node is one of the anchor nodes
                    if anchor_hit {
                        //Calculate gap cost
                        //circular stuff
                        //
                        let query_dist;
                        //                        query_dist = anchors[current_anchor_id].1 as i64
                        //                            - anchors[current_anchor_id - 1].1 as i64;
                        query_dist = (query_nodes[anchors[current_anchor_id].1 as usize]
                            .actual_ref_positions[0] as i64
                            - query_nodes[anchors[current_anchor_id - 1].1 as usize]
                                .actual_ref_positions[0] as i64)
                            .abs();

                        //                        let ref_dist = parent_path.2 + 1;
                        let ref_dist = parent_path.2 + bp_ref_distance;
                        let gap_cost = ((query_dist as i64).abs() - ref_dist as i64).abs() as f64;
                        let new_score_add = (5. - gap_cost) as f64;
                        let updated_path =
                            (new_color, parent_path.1 + new_score_add, 0, parent_path.3);
                        best_node_paths.push(updated_path);
                    }
                    //Just update the paths by increasing the distance by 1
                    else {
                        let updated_path =
//                            (new_color, parent_path.1, parent_path.2 + 1, parent_path.3);
                            (new_color, parent_path.1, parent_path.2 + bp_ref_distance, parent_path.3);
                        best_node_paths.push(updated_path);
                    }
                }
            }
        }

        if anchor_hit {
            let curr_best_path = best_node_paths
                .iter()
                .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap());
            if !curr_best_path.is_none() {
                if curr_best_path.unwrap().1 > cumulative_best_score {
                    best_node = intermediate_node;
                    cumulative_best_score = curr_best_path.unwrap().1
                }
            }
            //            dbg!(&intermediate_node);
            //            for path in best_node_paths.iter(){
            //                dbg!(align::get_nonzero_bits(path.0), path.1);
            //            }
        }

        if intermediate_node.color & all_colors != intermediate_node.color && parent_vec.len() > 0 {
            let remaining_colors = intermediate_node.color ^ all_colors;
            let mut lowest_score = 0.0;
            for path in best_node_paths.iter() {
                if lowest_score < path.1 {
                    lowest_score = path.1;
                }
            }
            if remaining_colors > 0 {
                //                println!("{:?}", &best_node_paths);
                //                print_as_binary(remaining_colors,"".to_string());
                //                print_as_binary(intermediate_node.color,"".to_string());
                //                print_as_binary(all_colors,"".to_string());
                best_node_paths.push((remaining_colors, lowest_score, 0, current_anchor_id));
            }
        }
        //        if intermediate_node.actual_ref_positions.len() > 0{
        //            dbg!(&best_node_paths, &intermediate_node);
        //        }

        let mut color_set = FxHashSet::default();
        for path in best_node_paths.iter() {
            if color_set.contains(&path.0) {
                dbg!(&best_node_paths, &intermediate_node);
                for id in in_edges_dict.get(&intermediate_node.id).unwrap() {
                    dbg!(&ref_nodes[*id as usize]);
                }
                panic!();
            } else {
                color_set.insert(path.0);
            }
        }
        //        if intermediate_node.id == 274837{
        //            dbg!(&best_node_paths, &intermediate_node);
        //        }
        best_paths.insert(intermediate_node.id, best_node_paths);
        if anchor_hit || unreachable_past {
            current_anchor_id += 1;
        }
    }

    let mut best_path_colors = vec![];
    let mut best_path_scores = vec![];
    let mut best_path_start_anchors = vec![];
    //    let best_path = best_paths.get(&last_node.id);
    let best_path = best_paths.get(&best_node.id);
    if let None = best_path {
        println!("Best path is None");
        return (vec![], vec![]);
    }
    if best_path.unwrap().is_empty() {
        println!("Best path is None");
        return (vec![], vec![]);
    }
    let best_path_score = best_path
        .unwrap()
        .iter()
        .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
        .unwrap()
        .1;

    if best_path_score < -25000.0 {
        println!("Best path is bad");
        return (vec![], vec![]);
    }

    for path in best_path.unwrap() {
        best_path_colors.push(path.0);
        best_path_scores.push(path.1);
        best_path_start_anchors.push(path.3);
    }

    let mut consistent_color_anchors = vec![];
    for j in 0..best_path_start_anchors.len() {
        let mut consistent_anchors = vec![];
        let start_anchor = best_path_start_anchors[j];
        for i in start_anchor..anchors.len() {
            let anchor = anchors[i];
            if &ref_nodes[anchor.0 as usize].color & best_path_colors[j] == best_path_colors[j] {
                consistent_anchors.push(anchor);
            }
        }
        consistent_color_anchors.push((consistent_anchors, best_path_scores[j]));
    }

    //    println!("{:?}", best_paths.get(&last_node.id));
    println!("{:?}", best_paths.get(&best_node.id));
    return (best_path_colors, consistent_color_anchors);
}

//pub fn get_best_path_from_chain(
//    anchors: Anchors,
//    ref_nodes: &Vec<KmerNode>,
//    order_to_id: &Vec<u32>,
//    q_len: u32,
//) -> (u64, Anchors) {
//    let mut in_edges_dict: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
//    let mut nodes_to_search = FxHashSet::default();
//    let mut best_paths: FxHashMap<u32, Vec<(u64, f64, usize)>> = FxHashMap::default();
//    let n = 25;
//    let mut current_anchor_id = 0;
//
//    if anchors.len() == 0 {
//        return (1, vec![]);
//    }
//
//    let last_node = &ref_nodes[anchors.last().unwrap().0 as usize];
//    let first_node = &ref_nodes[anchors[0].0 as usize];
//
//    //Create a starting path for the first node
//    for child_id in first_node.child_nodes.iter() {
//        nodes_to_search.insert(child_id);
//        let parent_vec = in_edges_dict.entry(*child_id).or_insert(vec![]);
//        parent_vec.push(first_node.id);
//    }
//
//    best_paths.insert(first_node.id, vec![(first_node.color, 10.0, 0)]);
//    current_anchor_id += 1;
//
//    for i in first_node.order + 1..last_node.order + 1 {
//        let id_of_node = order_to_id[i as usize];
//        //        println!("Id/order of node {},{}", id_of_node, i);
//
//        //Don't iterate over nodes which do not have an ancestor
//        if !nodes_to_search.contains(&id_of_node) {
//            if i == last_node.order {
//                //Edge case needs to be sorted out
//                //                dbg!(&first_node,&last_node);
//                //                dbg!(&ref_nodes[first_node.child_nodes[0] as usize]);
//                //                dbg!(&anchors);
//            }
//            continue;
//        }
//
//        let intermediate_node = &ref_nodes[id_of_node as usize];
//
//        for child_id in intermediate_node.child_nodes.iter() {
//            nodes_to_search.insert(child_id);
//            let parent_vec = in_edges_dict.entry(*child_id).or_insert(vec![]);
//            parent_vec.push(intermediate_node.id);
//        }
//
//        //Reevaluate paths
//        let mut best_node_paths = vec![];
//        best_node_paths.reserve(20);
//        let parent_vec = in_edges_dict.get(&intermediate_node.id).unwrap();
//        let anchor_hit;
//        let mut unreachable_past = false;
//
//        if anchors[current_anchor_id].0 == intermediate_node.id {
//            anchor_hit = true;
//        } else {
//            anchor_hit = false;
//            if ref_nodes[anchors[current_anchor_id].0 as usize].order < intermediate_node.order {
//                unreachable_past = true;
//            }
//        }
//
//        //Weird stuff happens w.r.t coloring if there is an insertion in the graph
//        //Consider    o
//        //          /   \
//        //        o   -  o
//        //
//        //        if the bottom-left is 111, top is 110, bottom-right is 111, then the path
//        //        from BL to BR should be 001.
//
//        //TODO change this when using more references.
//
//        let mut all_colors = 0;
//        for parent_id in parent_vec.iter() {
//            let parent_paths = best_paths.get(parent_id).unwrap();
//            for parent_path in parent_paths {
//                let parent_path_color = parent_path.0;
//                all_colors |= parent_path_color;
//                //Color coherence is needed for new paths w.r.t intermediate node
//                if parent_path_color & intermediate_node.color != 0 {
//                    let new_color = parent_path_color & intermediate_node.color;
//                    //If the intermediate node is one of the anchor nodes
//                    if anchor_hit {
//                        //Calculate gap cost
//                        //circular stuff
//                        //
//                        let query_dist;
//                        //                        if anchors[current_anchor_id].1 < anchors[current_anchor_id - 1].1 {
//                        //                            query_dist = anchors[current_anchor_id].1 + q_len
//                        //                                - anchors[current_anchor_id - 1].1;
//                        //                        } else {
//                        query_dist = anchors[current_anchor_id].1 as i64
//                            - anchors[current_anchor_id - 1].1 as i64;
//                        //                        }
//                        let ref_dist = parent_path.2 + 1;
//                        let gap_cost = 1 * ((query_dist as i64).abs() - ref_dist as i64).abs();
//
//                        let new_score_add = (10 - gap_cost) as f64;
//                        let updated_path = (new_color, parent_path.1 + new_score_add, 0);
//                        best_node_paths.push(updated_path);
//                    }
//                    //Just update the paths by increasing the distance by 1
//                    else {
//                        let updated_path = (new_color, parent_path.1, parent_path.2 + 1);
//                        best_node_paths.push(updated_path);
//                    }
//                }
//            }
//        }
//
//        if intermediate_node.color & all_colors != intermediate_node.color {
//            let remaining_colors = intermediate_node.color ^ all_colors;
//            if remaining_colors > 0 {
//                //                println!("{:?}", &best_node_paths);
//                //                print_as_binary(remaining_colors,"".to_string());
//                //                print_as_binary(intermediate_node.color,"".to_string());
//                //                print_as_binary(all_colors,"".to_string());
//                best_node_paths.push((remaining_colors, 0.0, 0));
//            }
//        }
//
//        //Update the path
//        //the xor operation is for the weird deletion case mentioned above.
//        if parent_vec.len() > 1 {
//            let mut test_set = FxHashSet::default();
//            let mut path_split = true;
//            best_node_paths.sort_by(|a, b| b.partial_cmp(&a).unwrap());
//            //            dbg!(&best_node_paths);
//
//            for path in best_node_paths.iter_mut() {
//                let x = path.0;
//                if !((x & (x - 1)) == 0) {
//                    //                    dbg!(x);
//                    path_split = false;
//                }
//                //collapse paths
//                if test_set.contains(&path.0) {
//                    path.0 = 0;
//                } else {
//                    test_set.insert(path.0);
//                }
//            }
//
//            if !path_split {
//                let mut changes = vec![];
//                for i in 0..best_node_paths.len() {
//                    let path_i = &best_node_paths[i];
//                    for j in i + 1..best_node_paths.len() {
//                        let path_j = &best_node_paths[j];
//                        if path_i.0 & path_j.0 == path_j.0 {
//                            changes.push((i, path_i.0 ^ path_j.0));
//                            break;
//                        }
//                    }
//                }
//
//                for (i, color) in changes {
//                    best_node_paths[i].0 = color;
//                }
//                //            if best_node_paths.len() > n{
//                //                best_node_paths.drain(n..);
//                //                    let parent_color= format!("{:#08b}", parent_path_color);
//                //                    let int_color= format!("{:#08b}", intermediate_node.color);
//                //                    dbg!(parent_color, int_color);
//                //            }
//                //            dbg!(&best_node_paths);
//            }
//        }
//        best_paths.insert(intermediate_node.id, best_node_paths);
//        if anchor_hit || unreachable_past {
//            current_anchor_id += 1;
//        }
//    }
//
//    let mut best_path_color = Color::MAX;
//    let mut best_path_score = f64::MIN;
//    let best_path = best_paths.get(&last_node.id);
//    if let None = best_path {
//        return (u64::MAX, vec![(0, 0)]);
//    }
//    for path in best_path.unwrap() {
//        if path.1 > best_path_score {
//            best_path_color = path.0;
//            best_path_score = path.1;
//        }
//    }
//
//    let mut consistent_color_anchors = vec![];
//    for anchor in anchors {
//        if &ref_nodes[anchor.0 as usize].color & best_path_color == best_path_color {
//            consistent_color_anchors.push(anchor);
//        }
//    }
//
//    println!("{:?}", best_paths.get(&last_node.id));
//    return (best_path_color, consistent_color_anchors);
//}

fn modulo_n(value: u32, n: u32, modulo_position: u32) -> usize {
    if value >= modulo_position {
        return (value - modulo_position) as usize;
    } else {
        if modulo_position > value + n {
            dbg!(value, n, modulo_position);
            panic!()
        }
        return (value + n - modulo_position) as usize;
    }
}

fn _get_mandatory_color(anchors: &Anchors, ref_graph: &Vec<KmerNode>) -> Color {
    //TODO
    let mut consensus_color = 0;
    let mut color_vec = vec![0; 128];
    for anchor in anchors {
        let color_node = ref_graph[anchor.0 as usize].color;
        for color in align::get_nonzero_bits(color_node) {
            color_vec[color] += 1;
        }
    }
    let mut best_colors: Vec<(usize, usize)> = color_vec.into_iter().enumerate().collect();
    best_colors.sort_by(|x, y| y.1.cmp(&x.1));
    let mut colors_to_ret = vec![];
    let best_score = best_colors[0].1;
    for (color, score) in best_colors {
        if score < best_score * 97 / 100 {
            break;
        }
        colors_to_ret.push(color);
    }

    println!("Mandatory: {:?}", &colors_to_ret);
    for color in colors_to_ret {
        consensus_color += Color::pow(2, color as u32);
    }
    return consensus_color;
}

fn score_anchors(
    f: &mut Vec<f64>,
    pointer_array: &mut [usize],
    anchors: &Anchors,
    chain_heuristic: bool,
    chain_reads: bool,
    h: usize,
    seeds_ref: &Vec<KmerNode>,
    seeds_q: &Vec<KmerNode>,
    modulo_positions: (u32, u32),
) {
    let q_len = seeds_q.len() as u32;
    let r_len = seeds_ref.len() as u32;
    let mut ref_color_vec = vec![0; anchors.len()];
    if chain_reads && chain_heuristic {
        for (i, anchor) in anchors.iter().enumerate() {
            ref_color_vec[i] = seeds_ref[anchor.0 as usize].color;
        }
    }
    //    let n = 1;
    let w;
    if chain_reads {
        w = 10.0;
    } else {
        w = 100.
    }
    let c1 = 1.0;

    //    let mut last_best_j = usize::MAX;
    let mut avl_tree: SearchTree<[usize; 2]> = SearchTree::new();
    if !chain_reads || !chain_heuristic {
        for (i, anchor) in anchors.iter().enumerate() {
            avl_tree.insert([
                modulo_n(seeds_q[anchor.1 as usize].order, q_len, modulo_positions.1),
                i,
            ]);
        }
        avl_tree.update_query_info(
            [
                modulo_n(
                    seeds_q[anchors[0].1 as usize].order,
                    q_len,
                    modulo_positions.1,
                ),
                0,
            ],
            c1 * (modulo_n(
                seeds_q[anchors[0].1 as usize].order,
                q_len,
                modulo_positions.1,
            ) + modulo_n(
                seeds_ref[anchors[0].0 as usize].order,
                r_len,
                modulo_positions.0,
            )) as f64,
            0,
            anchors[0].0 as usize,
            anchors[0].1 as usize,
        );
    }
    let mandatory_color = Color::MAX;

    for i in 1..anchors.len() {
        let anchoriq = modulo_n(
            seeds_q[anchors[i].1 as usize].order,
            q_len,
            modulo_positions.1,
        );
        let anchorir = modulo_n(
            seeds_ref[anchors[i].0 as usize].order,
            r_len,
            modulo_positions.0,
        );
        //        let mut best_f_i = usize::MIN as f64;
        let mut best_f_i = 0. as f64;
        let mut best_j = usize::MAX;
        let mut start = 0;
        if chain_heuristic && chain_reads {
            if i > h {
                start = i - h
            }
            for j in start..i {
                let anchorjq = seeds_q[anchors[j].1 as usize].order as usize;
                let anchorjr = seeds_ref[anchors[j].0 as usize].order as usize;
                let anchoriq = seeds_q[anchors[i].1 as usize].order as usize;

                if anchoriq < anchorjq {
                    //panic!("Don't deal with circular mappings right now");
                    //                    anchoriq = q_len as usize + anchoriq;
                }
                let anchorir = seeds_ref[anchors[i].0 as usize].order as usize;

                let color_history_jr = ref_color_vec[j];
                let color_ir = seeds_ref[anchors[i].0 as usize].color;
                //                if anchorir >= anchorjr{
                //                    anchorir = r_len as usize + anchorir;
                //                }

                //                println!("anchors {}, {}, {}, {}",anchorjq,anchorjr,anchoriq,anchorir);

                //                if start != 0 && j == start && last_best_j != usize::MAX {
                //                    j = last_best_j;
                //                }

                let mut incompat_chain = false;

                if anchorjr >= anchorir {
                    incompat_chain = true;
                }

                //This forces the chain to be a walkable path in the DAG
                if color_history_jr & color_ir == 0 {
                    incompat_chain = true;
                }

                if anchorjq >= anchoriq {
                    incompat_chain = true;
                }

                let f_cand_i;

                if incompat_chain {
                    f_cand_i = f64::MIN;
                } else {
                    let ref_order_dist = (anchorir - anchorjr) as f64;
                    let query_order_dist = (anchoriq - anchorjq) as f64;
                    f_cand_i = f[j] + heuristic_score(ref_order_dist, query_order_dist)
                }
                if f_cand_i > best_f_i {
                    best_f_i = f_cand_i;
                    best_j = j;
                }
            }
        } else {
            let gap_start = 0;
            let (best_score, best_id) = avl_tree.mrq(
                [gap_start as usize, 0],
                [anchoriq, i],
                anchors[i].0 as usize,
                anchors[i].1 as usize,
            );

            if best_score == i64::MIN {
                best_f_i = 0.0;
                best_j = i;
            } else {
                best_j = best_id;
                if chain_reads {
                    best_f_i = best_score as f64 - c1 * (anchorir + anchoriq) as f64 + w;
                } else {
                    best_f_i = best_score as f64 - c1 * (anchorir + anchoriq) as f64 + w;
                }

                if best_f_i < 0.0 {
                    best_f_i = 0.0;
                    best_j = i;
                }
            }
            if anchoriq
                < modulo_n(
                    seeds_q[anchors[best_j].1 as usize].order,
                    q_len,
                    modulo_positions.1,
                )
            {
                dbg!(&anchors[i], &anchors[best_j]);
                panic!()
            }
        }

        //        last_best_j = best_j;
        if best_f_i <= 0.0 {
            best_j = i
        }
        f.push(best_f_i);
        if !chain_heuristic || !chain_reads {
            avl_tree.update_query_info(
                [anchoriq, i],
                best_f_i + w + c1 * (anchorir + anchoriq) as f64,
                i,
                anchors[i].0 as usize,
                anchors[i].1 as usize,
            );
        }
        if best_j != usize::MAX {
            pointer_array[i] = best_j;
            //TODO
            //            ref_color_vec[i] &= ref_color_vec[best_j];
        }
    }
}

fn get_best_chains(
    f: Vec<f64>,
    pointer_array: Vec<usize>,
    anchors: &Anchors,
    seeds_ref: &Vec<KmerNode>,
    seeds_q: &Vec<KmerNode>,
    chain_reads: bool,
) -> Vec<(Anchors, (u32, u32), (u32, u32), f64)> {
    //    for anchor in anchors.iter(){
    //        dbg!(align::get_nonzero_bits(seeds_ref[anchor.0 as usize].color));
    //        dbg!(&seeds_ref[anchor.0 as usize]);
    //    }
    let mut vec: Vec<_> = f.iter().enumerate().collect();
    vec.sort_by(|(_, v0), (_, v1)| v1.partial_cmp(v0).unwrap());
    let mut return_chains: Vec<(Anchors, (u32, u32), (u32, u32), f64)> = vec![];
    let mut already_used_anchors = FxHashSet::default();
    let best_chain_score = f[vec[0].0];
    let cutoff_percent = 0.90;
    let cutoff_score = 300.;
    if best_chain_score < cutoff_score {
        println!("Poor chaining score {}", best_chain_score);
        return vec![];
    }
    for i in 0..vec.len() {
        let mut coherent_color = Color::MAX;
        //Only get the best chain for genome chaining
        if (i > 0 && !chain_reads) {
            break;
        }

        if already_used_anchors.contains(&i) {
            continue;
        }

        let mut best_seq_anchors = vec![];
        let mut chain_sequence = vec![];
        let mut curr_i = vec[i].0;
        let ith_score = f[curr_i];

        if ith_score < cutoff_percent * best_chain_score {
            break;
        }

        let mut prev_i = pointer_array[curr_i];
        chain_sequence.push(curr_i);

        while curr_i != prev_i {
            chain_sequence.push(prev_i);
            already_used_anchors.insert(curr_i);
            curr_i = prev_i;
            prev_i = pointer_array[curr_i];
        }
        already_used_anchors.insert(curr_i);

        let chain_end_ref = seeds_ref[anchors[chain_sequence[0]].0 as usize].order;
        let chain_start_ref =
            seeds_ref[anchors[chain_sequence[chain_sequence.len() - 1]].0 as usize].order;

        let mut secondary = false;
        for tup in return_chains.iter() {
            let range_ref = tup.1;
            if chain_start_ref >= range_ref.0 && chain_start_ref <= range_ref.1 {
                secondary = true
            }
            if range_ref.0 >= chain_start_ref && range_ref.0 <= chain_end_ref {
                secondary = true
            }
        }

        //TODO Don't output secondary chains for now.
        if secondary {
            //            println!("Secondary chain!")
            continue;
        }

        for i in (0..chain_sequence.len()).rev() {
            //       dbg!(anchors[chain_sequence[i]], pos1[anchors[chain_sequence[i]].0.order]);
            best_seq_anchors.push((anchors[chain_sequence[i]].0, anchors[chain_sequence[i]].1));
            coherent_color &= seeds_ref[anchors[chain_sequence[i]].0 as usize].color;
        }

        let color;
        if chain_reads && coherent_color != 0 {
            color = coherent_color
        } else {
            color = 1;
        }

        //    dbg!(f[best_i]);

        println!(
            "Chain number {}, Chain score {}, Color {:?}, Length {}",
            i,
            ith_score,
            align::get_nonzero_bits(color),
            best_seq_anchors.len()
        );
        println!(
            "End/start of ref anchors: {},{}",
            seeds_ref[anchors[chain_sequence[0]].0 as usize].order,
            seeds_ref[anchors[chain_sequence[chain_sequence.len() - 1]].0 as usize].order
        );
        println!(
            "End/start of query anchors: {},{}",
            seeds_q[anchors[chain_sequence[0]].1 as usize].order,
            seeds_q[anchors[chain_sequence[chain_sequence.len() - 1]].1 as usize].order
        );
        //    println!(
        //        "Ref/Query order anchor dist {},{}",
        //        (anchors[chain_sequence[0]].0.order)
        //            - (anchors[chain_sequence[chain_sequence.len() - 1]].0.order),
        //        (anchors[chain_sequence[0]].1.order)
        //            - (anchors[chain_sequence[chain_sequence.len() - 1]].1.order),
        //    );
        let first_anchor = &anchors[chain_sequence[chain_sequence.len() - 1]];
        let last_anchor = &anchors[chain_sequence[0]];
        let range_ref = (
            seeds_ref[first_anchor.0 as usize].order,
            seeds_ref[last_anchor.0 as usize].order,
        );
        let range_query = (
            seeds_q[first_anchor.1 as usize].order,
            seeds_q[last_anchor.1 as usize].order,
        );
        return_chains.push((best_seq_anchors, range_ref, range_query, ith_score));
    }

    return return_chains;
}
