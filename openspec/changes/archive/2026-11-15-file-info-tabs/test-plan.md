# Test Plan

Every scenario in this change is automated. Coverage names the Rust test
(`pictura-codec::<fn>`) or Qt Test slot (`tst_file_info::<case>`). Tests are
written in the same task group as the code they cover and run at the build gate.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The fixture's GPS values decode | Read the EXIF resource of `metadata.psd` | Latitude, longitude, altitude, timestamp, date, and datum match the stored GPS IFD, with the hemisphere applied | `pictura-codec::exif_gps_decodes_the_fixture_values` |
| A truncated GPS IFD does not panic | Cut the GPS IFD short and point an offset past the resource | Decoding returns the values read so far and never panics | `pictura-codec::exif_gps_survives_a_truncated_ifd` |
| No GPS IFD yields no GPS values | Decode an EXIF resource with no `0x8825` entry | No GPS result is present and the other entries are unchanged | `pictura-codec::exif_gps_absent_without_the_pointer` |
| Attribute-form and element-form properties both read | Read a packet with one attribute property and one element property | Both leaves are returned with their values | `pictura-codec::xmp_reads_attribute_and_element_forms` |
| Prefixes are resolved from the declarations in scope | Read a packet that binds the same URI to two prefixes | Both leaves are reported under the namespace URI | `pictura-codec::xmp_resolves_namespace_prefixes` |
| A bag and a struct read as separate leaves | Read a packet with a bag and a one-level struct | The bag yields one value per item and the struct one value per leaf | `pictura-codec::xmp_reads_bag_and_struct_leaves` |
| A malformed XMP packet does not panic | Truncate the packet inside an element | The reader returns the leaves decoded so far without panicking | `pictura-codec::xmp_malformed_packet_reads_what_it_can` |
| Patching a leaf preserves an unknown namespace | Patch one leaf in a packet carrying an unknown namespace | The leaf changes and the unknown bytes are identical | `pictura-codec::xmp_patch_preserves_unknown_namespace_bytes` |
| A missing bag is created | Set a bag property that is absent | A bag with one item per value is inserted and reads back | `pictura-codec::xmp_patch_creates_a_bag` |
| A missing struct is created with a local namespace declaration | Set a struct leaf where the namespace is not declared | The element carries a local `xmlns` and the leaf reads back | `pictura-codec::xmp_patch_creates_a_struct_with_local_namespace` |
| An unsupported shape is left unchanged | Patch a property with an unrecognised construct | The operation fails and the resource section is byte-identical | `pictura-codec::xmp_patch_fails_closed_on_unsupported_shape` |
| Removing a property removes only its bytes | Remove a leaf property | The property is absent and the surrounding bytes are unchanged | `pictura-codec::xmp_patch_removes_only_the_property` |
| The fixture's extension fields read | Read the fixture's XMP extension properties | Person bag, event, location city, and rating rows are returned | `pictura-codec::iptc_extension_reads_the_fixture_fields` |
| A missing extension namespace yields no rows | Read a packet with no Iptc4xmpExt property | The extension read rows are empty and no error is raised | `pictura-codec::iptc_extension_missing_namespace_is_empty` |
| A malformed extension packet does not panic | Truncate the packet inside an extension struct | Reading returns the rows decoded so far and does not panic | `pictura-codec::iptc_extension_malformed_packet_does_not_panic` |
| A person is added to the bag | Set Person Shown to two values where it was absent | The packet carries a two-item bag and reads back both | `pictura-codec::iptc_extension_adds_a_person_bag` |
| A location struct leaf is edited | Change the City leaf of Location Created | The new value reads back and the other leaves are unchanged | `pictura-codec::iptc_extension_edits_a_location_leaf` |
| Clearing a field removes it | Set an extension field to an empty value list | The property is absent and the packet stays well formed | `pictura-codec::iptc_extension_clears_a_field` |
| Unknown extension properties survive an edit | Edit a supported field beside an unknown extension property | The unknown property bytes are identical | `pictura-codec::iptc_extension_preserves_unknown_extension_bytes` |
| An unsupported field aborts the apply | Request one supported and one unsupported-shape field | No field is applied, the packet is unchanged, and the operation reports failure | `pictura-codec::iptc_extension_unsupported_shape_aborts` |
| Artwork leaves are listed without an edit path | Read a packet with an Artwork or Object structure | Its leaves appear as read-only rows and no writer targets them | `pictura-codec::iptc_extension_lists_artwork_read_only` |
| An unsupported structure survives a supported edit | Edit a supported field beside an artwork structure | The artwork bytes are identical | `pictura-codec::iptc_extension_artwork_survives_edit` |
| Additional location structures are read-only | Read a packet with two Location Shown structures | The first is editable and the rest are read-only rows | `pictura-codec::iptc_extension_lists_extra_structures_read_only` |
| The fixture's XMP packet is returned as text | Read the fixture's metadata | The raw string equals resource 1060's bytes decoded as text | `pictura-codec::metadata_decodes_and_agrees_with_exiftool` |
| The parsed properties accompany the raw string | Read the fixture's metadata | The parsed title, creator, description, rights, city, and usage terms are present beside the raw string | `pictura-codec::parsed_xmp_agrees_with_exiftool` |
| read_metadata agrees with exiftool | Decode `metadata.psd` with both decoders | EXIF, GPS, and IPTC values match | `pictura-codec::metadata_decodes_and_agrees_with_exiftool` |
| The fixture's extension fields agree with exiftool | Decode the fixture's XMP with both decoders | Person, event, location city, and rights usage terms match; the engine-side rating is 4 | `pictura-codec::parsed_extension_fields_agree_with_exiftool` |
| The oracle self-skips without exiftool | Run the extension oracle without `exiftool` | The test reports a skip and the suite passes | `pictura-codec::parsed_extension_fields_agree_with_exiftool` |
| File Info is disabled without a document | Open no document and inspect the command | `File > File Info…` is disabled | `tst_file_info::fileInfoDisabledWithoutDocument` |
| Opening File Info shows the editable categories | Open the metadata fixture and build the dialog from its rows | Eight categories appear in order; Camera Data shows Make, GPS shows the latitude, and no Video/Audio/DICOM tab exists | `tst_file_info::categoriesInCs6Order` |
| Editing a field and accepting applies one undo state | Change Author and accept | XMP creator and IIM By-line both change, the document is dirty, and one undo restores both | `tst_file_info::descriptionFieldWritesBothChannels` |
| Editing an Origin field writes both channels | Change City and accept | IIM City and XMP city both change | `tst_file_info::originFieldWritesBothChannels` |
| Editing an IPTC Extension field writes XMP only | Add a person and accept | The bag changes, the IIM resource is unchanged, and one undo restores it | `tst_file_info::extensionFieldWritesXmpOnly` |
| A shared field is mirrored across categories | Change Object Name in IPTC | The Document Title field in Description shows the same text | `tst_file_info::sharedFieldMirrorsAcrossCategories` |
| Editing Raw Data and a field is one undo state | Change the packet and a Description field, then accept | Both edits apply, one state is recorded, and one undo restores both | `tst_file_info::rawAndFieldEditAreOneUndoState` |
| An ill-formed raw packet applies nothing | Enter unbalanced raw XMP and accept | Nothing applies, no state is recorded, and the dialog stays open | `tst_file_info::illFormedRawPacketKeepsDialogOpen` |
| A missing value still shows an editable control | Open File Info on a fixture with no City | The Origin City control exists and is empty | `tst_file_info::missingValueKeepsEditableControl` |
| An unwritable extension field is refused and reported | Request an extension field whose packet shape cannot be rewritten and accept | The XMP resource is unchanged, no state is recorded, and the dialog stays open | `pictura-codec::iptc_extension_unsupported_shape_aborts`, `tst_file_info::extensionRefusalKeepsDialogOpen` |
| Cancelling applies nothing | Change a field and reject | Metadata is unchanged and the document is not dirtied by the dialog | `tst_file_info::cancelAppliesNothing` |
| Accepting with no changes records nothing | Accept without editing | No state is recorded and the document is not dirtied by the dialog | `tst_file_info::acceptWithNoChangesRecordsNothing` |
| The fixture's XMP yields its core properties | Parse the fixture's XMP | Title, creator, description, rights, city, state, country, instructions, authors position, caption writer, web statement, and usage terms decode | `pictura-codec::parsed_xmp_agrees_with_exiftool` |
| An element-form and an attribute-form property both decode | Parse a packet with one property per form | Both decode to their text | `pictura-codec::decodes_attribute_and_element_forms` |
| A truncated packet does not panic | Cut the packet inside an element | Parsing returns the properties read so far | `pictura-codec::malformed_and_truncated_never_panic` |
| An edited core field is written to both channels | Set Object Name | XMP title and IIM Object Name both change | `pictura-codec::shared_field_writes_both_channels` |
| An Origin field is written to both channels | Set City | XMP city and IIM City both change | `pictura-codec::metadata_origin_fields_sync_both_channels` |
| Keywords are written as a list | Set two keywords | XMP subject has two items and IIM has two Keywords datasets | `pictura-codec::metadata_keywords_sync_as_a_list` |
| An unchanged field changes neither channel | Set the values already held | Returns false and both resources are unchanged | `pictura-codec::metadata_unchanged_shared_field_is_a_noop` |
| A valid packet replaces resource 1060 | Supply a well-formed packet | The packet reads back and every other resource is identical | `pictura-codec::metadata_raw_packet_replaces_the_resource` |
| An ill-formed packet is rejected | Supply unbalanced markup | Returns false and the resource section is unchanged | `pictura-codec::metadata_raw_packet_rejects_malformed` |
| An empty packet removes the resource | Supply an empty packet | No XMP resource remains and other resources are unchanged | `pictura-codec::metadata_raw_packet_empty_removes` |
| Serialize then parse round-trips | Serialize a set with title, two creators, marked, and an Origin field | Parsing recovers the same set | `pictura-codec::template_serializes_the_expanded_managed_set` |
| An empty set round-trips | Serialize an empty set | Parsing yields empty properties | `pictura-codec::an_empty_set_serializes_to_an_empty_packet` |
| An applied core field reaches both channels | Apply a template with a title | XMP title and IIM Object Name both hold it | `pictura-codec::applied_template_survives_save_and_exiftool` |
| An applied Origin field reaches both channels | Apply a template with a City | XMP city and IIM City both hold it | `pictura-codec::template_origin_field_reaches_both_channels` |
| An applied keyword list reaches both channels | Apply a template with two keywords | XMP subject has two items and IIM has two datasets | `pictura-codec::template_keyword_list_reaches_both_channels` |
