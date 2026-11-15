//! The Zoomify tile layout: tiers of 256-pixel tiles, 256 tiles per
//! `TileGroup`, an `ImageProperties.xml` descriptor, and a self-contained HTML
//! viewer. The app renders the JPEG tiles through the Qt boundary; this module
//! owns the geometry and the text so both are unit-testable without Qt.

/// The Zoomify tile edge, in pixels.
pub const TILE: u32 = 256;
/// Tiles per `TileGroup` directory.
pub const TILES_PER_GROUP: u32 = 256;
/// The descriptor version Zoomify's published format uses.
pub const VERSION: &str = "1.8";

/// One tile cell inside a tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    /// Tile column in the tier.
    pub x: u32,
    /// Tile row in the tier.
    pub y: u32,
    /// The covered pixels inside the tier.
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
    /// Zero-based position across the whole pyramid, tier 0 first.
    pub ordinal: u32,
}

/// One resolution tier. Tier 0 is the smallest; the last tier is the base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tier {
    pub level: u32,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Tile>,
}

/// The full tile plan for a base image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub base_width: u32,
    pub base_height: u32,
    pub tiers: Vec<Tier>,
}

impl Plan {
    /// The total number of JPEG tiles.
    pub fn tile_count(&self) -> u32 {
        self.tiers.iter().map(|tier| tier.tiles.len() as u32).sum()
    }

    /// The plan flattened for the C++ boundary: `[base_w, base_h, tiers,
    /// tiles]`, then per tier `[level, width, height, tiles]`, then per tile
    /// `[x, y, left, top, width, height]`.
    pub fn flat(&self) -> Vec<i32> {
        let mut out = vec![
            self.base_width as i32,
            self.base_height as i32,
            self.tiers.len() as i32,
            self.tile_count() as i32,
        ];
        for tier in &self.tiers {
            out.extend([
                tier.level as i32,
                tier.width as i32,
                tier.height as i32,
                tier.tiles.len() as i32,
            ]);
            for tile in &tier.tiles {
                out.extend([
                    tile.x as i32,
                    tile.y as i32,
                    tile.left as i32,
                    tile.top as i32,
                    tile.width as i32,
                    tile.height as i32,
                ]);
            }
        }
        out
    }
}

fn ceil_div(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor)
}

/// The Zoomify plan for a `base_width × base_height` image. A zero dimension
/// yields an empty plan; the dialog refuses it before this point.
pub fn plan(base_width: u32, base_height: u32) -> Plan {
    if base_width == 0 || base_height == 0 {
        return Plan {
            base_width,
            base_height,
            tiers: Vec::new(),
        };
    }
    let longest = base_width.max(base_height);
    let mut levels = 1u32;
    let mut probe = longest;
    while probe > TILE {
        probe = probe.div_ceil(2);
        levels += 1;
    }

    let mut tiers = Vec::with_capacity(levels as usize);
    let mut ordinal = 0u32;
    for level in 0..levels {
        let divisor = 1u32 << (levels - 1 - level);
        let width = ceil_div(base_width, divisor);
        let height = ceil_div(base_height, divisor);
        let cols = ceil_div(width, TILE);
        let rows = ceil_div(height, TILE);
        let mut tiles = Vec::with_capacity((cols * rows) as usize);
        for y in 0..rows {
            for x in 0..cols {
                let left = x * TILE;
                let top = y * TILE;
                tiles.push(Tile {
                    x,
                    y,
                    left,
                    top,
                    width: (width - left).min(TILE),
                    height: (height - top).min(TILE),
                    ordinal,
                });
                ordinal += 1;
            }
        }
        tiers.push(Tier {
            level,
            width,
            height,
            tiles,
        });
    }
    Plan {
        base_width,
        base_height,
        tiers,
    }
}

/// The `TileGroup` directory index for a tile ordinal.
pub fn group_for(ordinal: u32) -> u32 {
    ordinal / TILES_PER_GROUP
}

/// The `ImageProperties.xml` descriptor.
pub fn image_properties(plan: &Plan) -> String {
    format!(
        "<IMAGE_PROPERTIES WIDTH=\"{}\" HEIGHT=\"{}\" NUMTILES=\"{}\" NUMIMAGES=\"1\" VERSION=\"{}\" TILESIZE=\"{}\" />\n",
        plan.base_width,
        plan.base_height,
        plan.tile_count(),
        VERSION,
        TILE
    )
}

/// A self-contained viewer page. It loads `<img>` tiles through relative
/// paths only and does not use `fetch`, XHR, or module imports, so it works
/// from `file://`.
pub fn viewer_html(plan: &Plan) -> String {
    if plan.tiers.is_empty() {
        return "<!doctype html>\n<html><body></body></html>\n".to_string();
    }
    let mut levels = String::new();
    let mut start = 0u32;
    for tier in &plan.tiers {
        let cols = ceil_div(tier.width, TILE);
        let rows = ceil_div(tier.height, TILE);
        if !levels.is_empty() {
            levels.push(',');
        }
        levels.push_str(&format!(
            "{{z:{},w:{},h:{},cols:{},rows:{},start:{}}}",
            tier.level, tier.width, tier.height, cols, rows, start
        ));
        start += cols * rows;
    }
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Zoomify</title>
<style>
html,body{{margin:0;height:100%;background:#111;overflow:hidden}}
#stage{{position:absolute;left:0;top:0;transform-origin:0 0;cursor:grab}}
#stage.drag{{cursor:grabbing}}
#stage img{{position:absolute;image-rendering:auto}}
#hint{{position:fixed;left:8px;bottom:8px;color:#ddd;font:12px sans-serif;background:#0008;padding:4px 6px}}
</style>
</head>
<body>
<div id="stage"></div>
<div id="hint">Drag to pan, wheel to zoom</div>
<script>
const W={w},H={h},TILE={tile},LEVELS=[{levels}];
let level=LEVELS.length-1,scale=1,panX=0,panY=0;
const stage=document.getElementById('stage');
function tileUrl(ordinal,z,x,y){{return 'TileGroup'+Math.floor(ordinal/{tiles_per_group})+'/'+z+'-'+x+'-'+y+'.jpg';}}
function show(next){{
  level=Math.max(0,Math.min(LEVELS.length-1,next));
  const L=LEVELS[level];
  stage.innerHTML='';
  stage.style.width=L.w+'px';stage.style.height=L.h+'px';
  for(let y=0;y<L.rows;y++)for(let x=0;x<L.cols;x++){{
    const img=document.createElement('img');
    img.src=tileUrl(L.start+y*L.cols+x,L.z,x,y);
    img.width=Math.min(TILE,L.w-x*TILE);img.height=Math.min(TILE,L.h-y*TILE);
    img.style.left=(x*TILE)+'px';img.style.top=(y*TILE)+'px';
    stage.appendChild(img);
  }}
  apply();
}}
function apply(){{stage.style.transform='translate('+panX+'px,'+panY+'px) scale('+scale+')';}}
function fit(){{
  const L=LEVELS[level];scale=Math.min(innerWidth/L.w,innerHeight/L.h,1);
  panX=(innerWidth-L.w*scale)/2;panY=(innerHeight-L.h*scale)/2;apply();
}}
let dragging=false,lastX=0,lastY=0;
stage.addEventListener('pointerdown',e=>{{dragging=true;lastX=e.clientX;lastY=e.clientY;stage.classList.add('drag');stage.setPointerCapture(e.pointerId);}});
stage.addEventListener('pointermove',e=>{{if(!dragging)return;panX+=e.clientX-lastX;panY+=e.clientY-lastY;lastX=e.clientX;lastY=e.clientY;apply();}});
stage.addEventListener('pointerup',()=>{{dragging=false;stage.classList.remove('drag');}});
addEventListener('wheel',e=>{{e.preventDefault();show(level+(e.deltaY>0?-1:1));fit();}},{{passive:false}});
addEventListener('resize',fit);
show(level);fit();
</script>
</body>
</html>
"#,
        w = plan.base_width,
        h = plan.base_height,
        tile = TILE,
        tiles_per_group = TILES_PER_GROUP,
        levels = levels
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoomify_plan_small_base_is_one_tile() {
        let plan = plan(200, 100);
        assert_eq!(plan.tiers.len(), 1);
        assert_eq!(plan.tile_count(), 1);
        assert_eq!(plan.tiers[0].tiles[0].width, 200);
        assert_eq!(plan.tiers[0].tiles[0].height, 100);
        assert!(image_properties(&plan).contains("NUMTILES=\"1\""));
    }

    #[test]
    fn zoomify_plan_multi_tier_tiles() {
        let plan = plan(1000, 700);
        let counts: Vec<u32> = plan
            .tiers
            .iter()
            .map(|tier| tier.tiles.len() as u32)
            .collect();
        assert_eq!(counts, vec![1, 4, 12]);
        assert_eq!(plan.tile_count(), 17);
        assert!(image_properties(&plan).contains("NUMTILES=\"17\""));
        let last = plan.tiers.last().unwrap();
        let bottom_right = last
            .tiles
            .iter()
            .find(|tile| tile.x == 3 && tile.y == 2)
            .unwrap();
        assert_eq!(bottom_right.width, 1000 - 3 * 256);
        assert_eq!(bottom_right.height, 700 - 2 * 256);
    }

    #[test]
    fn zoomify_tile_group_rolls_over() {
        let plan = plan(4096, 4096);
        assert!(plan.tile_count() > 256);
        let tile = plan
            .tiers
            .iter()
            .flat_map(|tier| &tier.tiles)
            .find(|tile| tile.ordinal == 256)
            .unwrap();
        assert_eq!(group_for(255), 0);
        assert_eq!(group_for(tile.ordinal), 1);
        assert_eq!(group_for(511), 1);
        assert_eq!(group_for(512), 2);
    }

    #[test]
    fn zoomify_edge_tiles_keep_full_cells() {
        let plan = plan(300, 200);
        let base = plan.tiers.last().unwrap();
        assert_eq!(base.level, 1);
        let tiles = &base.tiles;
        assert_eq!(tiles.len(), 2);
        assert_eq!(tiles[1].x, 1);
        assert_eq!(tiles[1].width, 44);
        assert_eq!(tiles[1].height, 200);
    }

    #[test]
    fn zoomify_viewer_html_references_tiles() {
        let html = viewer_html(&plan(1000, 700));
        assert!(html.contains("TileGroup"));
        assert!(html.contains("W=1000"));
        assert!(html.contains("H=700"));
        assert!(html.contains("LEVELS=["));
        assert!(html.contains("{z:2,w:1000,h:700,cols:4,rows:3,start:5}"));
    }

    #[test]
    fn zoomify_viewer_html_is_self_contained() {
        let html = viewer_html(&plan(1000, 700));
        for forbidden in [
            "http://",
            "https://",
            "fetch(",
            "XMLHttpRequest",
            "import ",
            "<script src",
            "<link ",
        ] {
            assert!(!html.contains(forbidden), "viewer contains {forbidden}");
        }
    }

    #[test]
    fn zoomify_flat_plan_matches_tiers() {
        let plan = plan(1000, 700);
        let flat = plan.flat();
        assert_eq!(&flat[..4], &[1000, 700, 3, 17]);
        assert_eq!(&flat[4..8], &[0, 250, 175, 1]);
    }
}
