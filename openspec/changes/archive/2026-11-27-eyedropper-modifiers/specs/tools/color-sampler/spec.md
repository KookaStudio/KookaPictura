# Spec Delta

## ADDED Requirements

### Requirement: Place a sampler with the Eyedropper

Shift-click with the Eyedropper SHALL place one Color Sampler at the clicked document pixel using the existing sampler limit and history behavior. The gesture SHALL leave foreground and background colours unchanged and SHALL NOT sample on drag. Shift takes precedence when Alt is also held.

#### Scenario: Shift-click places a sampler [eye_sampler]
- **WHEN** the Eyedropper Shift-clicks a document pixel
- **THEN** one sampler is added at that pixel and neither colour changes

#### Scenario: Fifth Shift-click is refused [eye_sampler_limit]
- **WHEN** four samplers exist and the Eyedropper Shift-clicks another pixel
- **THEN** the sampler count and history remain unchanged and the refusal is reported

#### Scenario: Shift and Alt together place a sampler [eye_sampler_priority]
- **WHEN** the Eyedropper is clicked with Shift and Alt held
- **THEN** one sampler is placed and neither colour changes
