use super::Raster2D;

/// Print raster in simple ascii
pub fn raster_to_str(raster: &Raster2D) -> String {
    (0..raster.height)
        .map(|y| {
            let to_row = |x| {
                if raster.get(x, y) { '8' } else { '.' }
            };
            (0..raster.width).map(to_row).collect::<String>()
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Print raster in braille
pub fn render_braille(raster: &Raster2D) -> String {
    if raster.width % 2 != 0 {
        panic!("Cannot render board if width is not a multiple of two");
    }
    if raster.height % 4 != 0 {
        panic!("Cannot render board if height is not a multiple of two");
    }

    let width = (raster.width / 2) as usize;
    let height = (raster.height / 4) as usize;

    let mut lines: Vec<Vec<[u8; 3]>> = vec![
        std::iter::repeat([0xe2u8, 0xa0u8, 0x80u8])
            .take(width)
            .collect();
        height
    ];
    for h in 0..raster.height {
        let vert_placement = h as usize % 4;
        for w in 0..raster.width {
            let horiz_placement = w as usize % 2;
            if raster.get(w, h) {
                let (second, third) = match (vert_placement, horiz_placement) {
                    (0, 0) => (0b00000000, 0b00000001),
                    (1, 0) => (0b00000000, 0b00000010),
                    (2, 0) => (0b00000000, 0b00000100),
                    (3, 0) => (0b00000001, 0b00000000),
                    (0, 1) => (0b00000000, 0b00001000),
                    (1, 1) => (0b00000000, 0b00010000),
                    (2, 1) => (0b00000000, 0b00100000),
                    (3, 1) => (0b00000010, 0b00000000),
                    m => panic!("Unexpected modulo of (%4, %2): {:?}", m),
                };
                lines[h as usize / 4][w as usize / 2][1] |= second;
                lines[h as usize / 4][w as usize / 2][2] |= third;
            }
        }
    }
    lines.into_iter().map(|line| {
        let l = line.into_iter().flatten().collect::<Vec<u8>>();
        std::str::from_utf8(&l).unwrap().to_owned()
    }).collect::<Vec<_>>().join("\n")
}
