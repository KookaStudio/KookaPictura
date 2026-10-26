# Test plan

Every scenario in the delta specs has a row. The Move rows drive the bridge
(`begin_move_preview` + `commit_move`), so they cover the drag without synthetic
mouse input.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The implemented strip buttons carry icons | Read the seven action-strip buttons' icons | Each matches its `layers.*` asset | `tst_linked_layers::stripButtonsCarryIcons` |
| lpl_strip_link | Select two layers, click the Link strip button, then click again | Both rows report linked after the first click, unlinked after the second | `tst_linked_layers::stripLinkButtonToggles` |
| lpc_strip_no_document | Clear the panel's view and read the strip buttons | All seven are disabled | `tst_layers_panel_fixes::bottomStripDisabledWithoutDocument` |
| lpl_row_link | Link a layer and read its row badge rect | The link badge rect is non-empty | `tst_linked_layers::linkedRowShowsChainBadge` |
| lpl_row_link_absent | Read an unlinked layer's row badge rect | The link badge rect is empty | `tst_linked_layers::unlinkedRowShowsNoChainBadge` |
| Dragging a linked layer moves the set | Link two pixel layers, begin the move preview, commit a delta | Both rects shift by the delta, one history state, and the preview reports the union origin | `tst_linked_layers::dragLinkedLayersMovesSet`, `pictura-app::translate_move_group_moves_every_member` |
| An unlinked layer moves alone | Move an unlinked layer | Only that layer shifts | `tst_linked_layers::dragUnlinkedLayerAlone` |
| A position-locked member is left in place | Link a position-locked layer with another, move the set | The locked layer stays, the other shifts | `tst_linked_layers::lockedMemberStaysPut`, `pictura-app::translate_move_group_skips_locked` |
