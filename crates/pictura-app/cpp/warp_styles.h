#pragma once

// The Warp style menu, in CS6 order: display name and the bridge id
// (`pictura_core::warp::WarpStyle::id`; keep the two lists in step). Free
// Transform's Warp mode lists every entry; Warp Text leaves out Custom.

namespace pictura {

struct WarpStyleEntry {
    const char* name;
    const char* id;
};

inline constexpr WarpStyleEntry kWarpStyles[] = {
    {"None", "none"},           {"Custom", "custom"},        {"Arc", "arc"},
    {"Arc Lower", "arcLower"},  {"Arc Upper", "arcUpper"},   {"Arch", "arch"},
    {"Bulge", "bulge"},         {"Shell Lower", "shellLower"}, {"Shell Upper", "shellUpper"},
    {"Flag", "flag"},           {"Wave", "wave"},            {"Fish", "fish"},
    {"Rise", "rise"},           {"Fisheye", "fisheye"},      {"Inflate", "inflate"},
    {"Squeeze", "squeeze"},     {"Twist", "twist"},
};

} // namespace pictura
