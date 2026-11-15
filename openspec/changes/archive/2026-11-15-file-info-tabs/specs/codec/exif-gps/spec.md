# Spec Delta

## Purpose

Decodes the EXIF GPS sub-IFD into typed geolocation values so File Info can show where a photograph was taken.

## ADDED Requirements

### Requirement: The EXIF GPS sub-IFD is decoded into typed values

`pictura-codec` SHALL decode the GPS sub-IFD (`0x8825`) of an EXIF image resource into typed values: latitude and longitude (degrees, minutes, and seconds rationals combined with their N/S and E/W references), altitude and its reference, a UTC timestamp, a date stamp, and the map datum. The decoder SHALL accept both the `Exif\0\0`-prefixed and bare-TIFF forms, SHALL bound the entries and value bytes it reads, SHALL NOT recurse, and SHALL NOT panic: a truncated or malformed GPS IFD SHALL yield the values decoded so far, and a resource without a GPS IFD SHALL yield no GPS values. The other EXIF entries SHALL be unchanged by GPS decoding.

#### Scenario: The fixture's GPS values decode

- **WHEN** the EXIF decoder reads the metadata fixture's EXIF resource
- **THEN** the GPS result carries the stored latitude, longitude, altitude, timestamp, date, and datum, with the hemisphere reference applied to the sign

#### Scenario: A truncated GPS IFD does not panic

- **WHEN** the GPS IFD is cut short or an entry offset points past the resource
- **THEN** decoding returns the values read so far and does not panic

#### Scenario: No GPS IFD yields no GPS values

- **WHEN** the EXIF resource has no `0x8825` entry
- **THEN** the GPS result is absent and the other EXIF entries are unchanged
