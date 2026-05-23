use crate::cell_group::{CellGroup, CellGroupType, CellGroups};
use crate::index::Index;
use rand::seq::IndexedRandom;
use rand::Rng;

use super::GridGenerator;

/// Generates valid Nonomino (jigsaw Sudoku) region layouts.
///
/// Each layout consists of 9 connected regions of exactly 9 cells each,
/// covering all 81 cells exactly once. Layouts are verified to support at
/// least one valid filled Sudoku grid before being returned.
pub struct NonominoRegionGenerator {
    /// Maximum number of generation attempts before giving up.
    pub max_attempts: usize,
}

impl Default for NonominoRegionGenerator {
    fn default() -> Self {
        Self { max_attempts: 200 }
    }
}

impl NonominoRegionGenerator {
    pub fn new(max_attempts: usize) -> Self {
        Self { max_attempts }
    }

    /// Generates a valid Nonomino region layout as a [`CellGroups`] containing
    /// 9 nonomino regions, 9 rows, and 9 columns. Returns `None` if all
    /// attempts are exhausted without a puzzle-capable layout.
    pub fn generate<R: Rng>(&self, rng: &mut R) -> Option<CellGroups> {
        for _ in 0..self.max_attempts {
            let assignment = randomize_regions(rng);
            let groups = build_cell_groups(&assignment);
            // Use a node budget to quickly reject degenerate layouts.
            if GridGenerator::new(groups.clone())
                .try_generate_limited(rng, 10_000)
                .is_some()
            {
                return Some(groups);
            }
        }
        None
    }
}

/// Generates a random region assignment via boundary-swap randomization.
///
/// Starts from the 9 standard 3x3 blocks (which are always valid: 9 connected
/// regions of exactly 9 cells each). Repeatedly swaps a boundary cell of
/// region A with a boundary cell of an adjacent region B, accepting only
/// swaps that keep both regions connected. Performs up to [`NUM_SWAPS`] accepted
/// swaps; may do fewer if `MAX_ATTEMPTS` candidate swaps are exhausted first.
///
/// This approach guarantees: size invariant (each region always has exactly 9
/// cells) and connectivity invariant (each region remains connected). No
/// backtracking or failure path exists.
fn randomize_regions<R: Rng>(rng: &mut R) -> [u8; 81] {
    const NUM_SWAPS: usize = 300;
    const MAX_ATTEMPTS: usize = NUM_SWAPS * 40;

    // Initialize with standard 3x3 blocks.
    let mut assigned = [0u8; 81];
    for y in 0..9usize {
        for x in 0..9usize {
            assigned[y * 9 + x] = ((y / 3) * 3 + (x / 3)) as u8;
        }
    }

    let all_cells: Vec<usize> = (0..81).collect();
    let mut swaps_done = 0;
    let mut tries = 0;

    while swaps_done < NUM_SWAPS && tries < MAX_ATTEMPTS {
        tries += 1;

        // Pick a random cell C1 as a candidate for the swap.
        let c1 = *all_cells.choose(rng).unwrap();
        let region_a = assigned[c1];

        // C1 must border some other region B.
        let b_neighbors: Vec<usize> = neighbors(c1).filter(|&n| assigned[n] != region_a).collect();
        if b_neighbors.is_empty() {
            continue;
        }

        // Choose region B from the adjacent regions.
        let region_b = assigned[*b_neighbors.choose(rng).unwrap()];

        // Find all cells of B that border region A — these are C2 candidates.
        let c2_candidates: Vec<usize> = (0..81)
            .filter(|&idx| {
                assigned[idx] == region_b && neighbors(idx).any(|n| assigned[n] == region_a)
            })
            .collect();
        if c2_candidates.is_empty() {
            continue;
        }
        let c2 = *c2_candidates.choose(rng).unwrap();

        // Skip degenerate swaps: C1 and C2 must be distinct.
        if c1 == c2 {
            continue;
        }

        // Attempt the swap.
        assigned[c1] = region_b;
        assigned[c2] = region_a;

        if is_region_connected(region_a, &assigned) && is_region_connected(region_b, &assigned) {
            swaps_done += 1;
        } else {
            // Undo.
            assigned[c1] = region_a;
            assigned[c2] = region_b;
        }
    }

    assigned
}

/// Returns `true` if all cells belonging to `region` form a single
/// 4-connected component.
fn is_region_connected(region: u8, assigned: &[u8; 81]) -> bool {
    let mut start = None;
    let mut total = 0usize;
    for (idx, &r) in assigned.iter().enumerate() {
        if r == region {
            total += 1;
            if start.is_none() {
                start = Some(idx);
            }
        }
    }
    let Some(start) = start else { return true };
    if total == 1 {
        return true;
    }

    let mut visited = [false; 81];
    let mut queue = vec![start];
    visited[start] = true;
    let mut count = 1usize;

    while let Some(cell) = queue.pop() {
        for n in neighbors(cell) {
            if assigned[n] == region && !visited[n] {
                visited[n] = true;
                count += 1;
                queue.push(n);
            }
        }
    }

    count == total
}

/// Returns the 4-connected neighbours of a cell index within the 9x9 grid.
fn neighbors(idx: usize) -> impl Iterator<Item = usize> {
    let x = (idx % 9) as i32;
    let y = (idx / 9) as i32;
    [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)]
        .into_iter()
        .filter_map(move |(dx, dy)| {
            let nx = x + dx;
            let ny = y + dy;
            if (0..9).contains(&nx) && (0..9).contains(&ny) {
                Some((ny * 9 + nx) as usize)
            } else {
                None
            }
        })
}

/// Converts a per-cell region assignment into a [`CellGroups`] with 9 nonomino
/// region groups, 9 row groups, and 9 column groups.
fn build_cell_groups(assignment: &[u8; 81]) -> CellGroups {
    let mut groups = CellGroups::default();

    for region in 0..9u8 {
        let mut group = CellGroup::new((region + 1) as usize, CellGroupType::Custom);
        for idx in 0..81u8 {
            if assignment[idx as usize] == region {
                group.add_index(Index::new(idx));
            }
        }
        groups.add_group(group);
    }

    groups.with_default_rows_and_columns()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn make_rng(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
    }

    #[test]
    fn generated_layout_covers_all_81_cells() {
        let mut rng = make_rng(1);
        let assignment = randomize_regions(&mut rng);
        let groups = build_cell_groups(&assignment);

        let region_groups: Vec<_> = groups
            .iter()
            .filter(|g| g.group_type == CellGroupType::Custom)
            .collect();
        assert_eq!(region_groups.len(), 9);
        for g in &region_groups {
            assert_eq!(g.len(), 9, "region has wrong size");
        }

        let mut covered = [false; 81];
        for g in &region_groups {
            for idx in g.iter_indexes() {
                let i = *idx as usize;
                assert!(!covered[i], "cell {i} assigned to multiple regions");
                covered[i] = true;
            }
        }
        assert!(covered.iter().all(|&c| c), "not all cells covered");
    }

    #[test]
    fn generated_layout_regions_are_connected() {
        let mut rng = make_rng(2);
        let assignment = randomize_regions(&mut rng);
        let groups = build_cell_groups(&assignment);

        for g in groups
            .iter()
            .filter(|g| g.group_type == CellGroupType::Custom)
        {
            let cells: Vec<usize> = g.iter_indexes().map(|i| *i as usize).collect();
            assert!(is_connected(&cells), "region is not 4-connected");
        }
    }

    #[test]
    fn different_seeds_produce_different_layouts() {
        let assignment_a = randomize_regions(&mut make_rng(10));
        let assignment_b = randomize_regions(&mut make_rng(11));
        assert_ne!(assignment_a, assignment_b);
    }

    #[test]
    fn generate_returns_none_when_max_attempts_is_zero() {
        let gen = NonominoRegionGenerator::new(0);
        let mut rng = make_rng(42);
        assert!(gen.generate(&mut rng).is_none());
    }

    #[test]
    fn new_and_default_set_max_attempts() {
        assert_eq!(NonominoRegionGenerator::new(42).max_attempts, 42);
        assert_eq!(NonominoRegionGenerator::default().max_attempts, 200);
    }

    #[test]
    #[ignore = "Nonomino grid generation is inherently slow in debug builds (many nodes \
                required). Run with --release or as part of integration testing."]
    fn generated_layout_supports_valid_grid() {
        let gen = NonominoRegionGenerator::new(200);
        let mut rng = make_rng(3);
        let groups = gen.generate(&mut rng).expect("generation should succeed");
        // generate() already verified puzzle-capability via try_generate_limited,
        // so reaching here confirms the layout supports at least one valid grid.
        let _ = groups;
    }

    #[test]
    fn randomize_regions_always_valid() {
        let mut rng = make_rng(99);
        for _ in 0..20 {
            let assignment = randomize_regions(&mut rng);
            assert!(
                assignment.iter().all(|&r| r < 9),
                "all cells must be assigned"
            );
            let mut counts = [0u8; 9];
            for &r in &assignment {
                counts[r as usize] += 1;
            }
            assert!(
                counts.iter().all(|&c| c == 9),
                "each region must have 9 cells"
            );
            for region in 0..9u8 {
                let cells: Vec<usize> = assignment
                    .iter()
                    .enumerate()
                    .filter(|(_, &r)| r == region)
                    .map(|(i, _)| i)
                    .collect();
                assert!(is_connected(&cells), "region {region} must be connected");
            }
        }
    }

    /// BFS connectivity check for a set of cell indexes.
    fn is_connected(cells: &[usize]) -> bool {
        if cells.is_empty() {
            return true;
        }
        let cell_set: std::collections::HashSet<usize> = cells.iter().copied().collect();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(cells[0]);
        visited.insert(cells[0]);
        while let Some(cur) = queue.pop_front() {
            for n in neighbors(cur) {
                if cell_set.contains(&n) && !visited.contains(&n) {
                    visited.insert(n);
                    queue.push_back(n);
                }
            }
        }
        visited.len() == cells.len()
    }
}
