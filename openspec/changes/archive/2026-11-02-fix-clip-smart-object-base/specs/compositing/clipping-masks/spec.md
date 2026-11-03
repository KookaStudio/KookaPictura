# Spec Delta

## ADDED Requirements

### Requirement: A smart-object base clips to its rendered alpha

When a clipping base is a smart object rendered from its embedded source, the
clipped layers SHALL be confined to that source's rendered alpha, including a
transparent margin inside the base's rect. A base rendered from its raster proxy
SHALL keep clipping to the proxy's rendered alpha.

#### Scenario: A proxy-less base with a transparent margin clips [clip_smart_source_base]

- **WHEN** a full-canvas layer is clipped to a proxy-less smart-object base whose
  embedded source has a transparent margin and an opaque area
- **THEN** the clipped colour appears only inside the opaque area and not over
  the transparent margin

#### Scenario: A raster-proxy base keeps its proxy alpha [clip_smart_proxy_base]

- **WHEN** a full-canvas layer is clipped to a smart-object base that carries a
  raster proxy with a transparent margin
- **THEN** the clipped colour appears only inside the proxy's opaque area
