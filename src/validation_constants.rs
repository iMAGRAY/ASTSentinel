/// Centralized validation constants for memory optimization
/// These constants define the boundaries and limits for various fields in the
/// memory structure
// Active context validation
pub const MAX_CURRENT_TASK_LENGTH: usize = 1000;
pub const MAX_LAST_ACTION_LENGTH: usize = 1000;
pub const MIN_NEXT_STEP_LENGTH: usize = 10;
pub const MAX_NEXT_STEP_LENGTH: usize = 500;
pub const MAX_NEXT_STEPS_COUNT: usize = 100;

// Technical detail validation
pub const MAX_TECHNICAL_DETAIL_CONTENT_LENGTH: usize = 5000;
pub const MAX_TECHNICAL_DETAIL_LOCATION_LENGTH: usize = 500;
pub const MAX_TECHNICAL_DETAIL_STATUS_LENGTH: usize = 100;

// Key insights validation
pub const MIN_KEY_INSIGHT_LENGTH: usize = 10;
pub const MAX_KEY_INSIGHT_LENGTH: usize = 1000;
pub const MAX_KEY_INSIGHTS_COUNT: usize = 50;

// Token limits
pub const MAX_TOTAL_TOKENS: usize = 10_000_000;
pub const WARN_TOTAL_TOKENS: usize = 1_000_000;

// Documentation references validation
pub const MAX_DOC_REF_FILE_PATH_LENGTH: usize = 500;
pub const MAX_DOC_REF_SECTION_LENGTH: usize = 200;
pub const MAX_DOC_REF_SUMMARY_LENGTH: usize = 1000;

// AI error patterns validation
pub const MAX_ERROR_PATTERN_TYPE_LENGTH: usize = 100;
pub const MAX_ERROR_PATTERN_PATTERN_LENGTH: usize = 500;
pub const MAX_ERROR_PATTERN_GUIDANCE_LENGTH: usize = 1000;

// Learning insights validation
pub const MAX_LEARNING_INSIGHT_CATEGORY_LENGTH: usize = 100;
pub const MAX_LEARNING_INSIGHT_INSIGHT_LENGTH: usize = 1000;
pub const MAX_LEARNING_INSIGHT_SOURCE_LENGTH: usize = 100;

// Completeness scoring
pub const TOTAL_MEMORY_SECTIONS: f64 = 12.0;
pub const MIN_COMPLETENESS_WARNING_THRESHOLD: f64 = 0.5;

// Path validation constants
pub const MAX_PATH_LENGTH: usize = 4096;
pub const MAX_PROJECT_NAME_LENGTH: usize = 255;

// Transcript and context constants
pub const MAX_TRANSCRIPT_SIZE: usize = 8000;
pub const DEFAULT_CONTEXT_WINDOW: usize = 8000;
