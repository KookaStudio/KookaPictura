# Design

`CpuHeatmap` is a paint-only widget. Cells are 22 px squares with a 2 px gap laid
out row-major; rows = 1 when the CPU count is 8 or fewer, else 2, and columns =
ceil(count / rows). Colour is a three-stop ramp over 0-100 %: blue `#3B6FB6`,
amber `#F2B134`, red `#C0281C`, each cell outlined dark so it reads on the yellow
panel. The loads are the per-CPU percentages the sampler already produces, so no new
source is read. The widget is emptied when the panel deactivates, like the charts.
