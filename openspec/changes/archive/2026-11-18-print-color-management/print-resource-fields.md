# Photoshop print resource fields

The public mappings below guide codec development. The Photoshop CS6 and CC
single-field comparisons and the final round-trip results are pending.

| Field | Resource and representation | Evidence |
|---|---|---|
| Colour handling | 1082 `printOutput`; `PstS` true for printer-managed colour | Public mapping |
| Printer profile | 1082 `Nm  ` profile name and `ClrS` enum with `RGBC` or `Grys` | Public mapping; CMYK enum pending |
| Rendering intent | 1082 `Inte` enum with `Img `, `Clrm`, `Grp `, `AClr` | Public mapping |
| Black-point compensation | 1082 `MpBl` boolean | Public mapping |
| Hard proof mode | 1082 `hardProof` boolean | Public mapping |
| Proof condition | 1082 `printProofSetup` object, class `proofSetup`. Built-in `Bltn` enum or named `profile` with `Inte`, `MpBl` and `paperWhite`. | Public mapping; separate black-ink wire field pending |
| Marks and description | 1083 `printOutputOptions`; `Clbr`, `RgsM`, `CrnC`, `CntC`, `Lbls`, `Cptn` booleans | Public mapping |
| Background, border and bleed | 1083 `Bckg` RGB object with 0–255 channels, `BrdT` and `Bld ` unit values | Public mapping |
| Legacy marks | 1011 byte flags and resource 10000 version-1 centre crop byte | Adobe file format and public mapping |

[Adobe PSD specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)
· [1082 mapping](https://github.com/Agamnentzar/ag-psd/blob/387049670cb89b88fb8fe1b7c01aeacf98dd2e3b/src/imageResources.ts#L127-L222)
· [1083 mapping](https://github.com/Agamnentzar/ag-psd/blob/master/src/imageResources.ts#L280-L372)
