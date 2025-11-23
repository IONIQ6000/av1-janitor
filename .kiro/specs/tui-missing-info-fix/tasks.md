# Implementation Plan

- [x] 1. Add metadata validation and formatting utilities
  - Create helper functions for validating job metadata completeness
  - Create consistent formatting functions for optional values
  - Add functions to identify missing metadata fields
  - _Requirements: 1.2, 4.2_

- [x] 1.1 Write property test for metadata validation
  - **Property 2: Missing metadata indicator accuracy**
  - **Validates: Requirements 1.2**

- [x] 2. Improve job table metadata display
  - Ensure all available metadata fields are displayed in table rows
  - Add clear "-" indicators for missing fields
  - Ensure codec names are consistently uppercased
  - Improve missing metadata indicators to show specific missing fields
  - _Requirements: 1.1, 1.2, 1.5_

- [x] 2.1 Write property test for metadata field display
  - **Property 1: Metadata field display completeness**
  - **Validates: Requirements 1.1**

- [x] 2.2 Write property test for codec formatting
  - **Property 4: Codec name formatting consistency**
  - **Validates: Requirements 1.5**

- [x] 3. Enhance current job panel with complete progress information
  - Ensure all progress fields are displayed (stage, percentage, speed, ETA, elapsed)
  - Add FPS display with validation (0.1-500 range)
  - Add estimated final size display
  - Add quality setting (CRF) display
  - Add validation to ensure progress percentage is 0-100
  - Add fallback "-" for uncalculable values
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 3.1 Write property test for progress display completeness
  - **Property 5: Running job progress display completeness**
  - **Validates: Requirements 2.1, 2.5**

- [x] 3.2 Write property test for FPS calculation
  - **Property 6: FPS calculation validity**
  - **Validates: Requirements 2.2**

- [x] 3.3 Write property test for running job estimation
  - **Property 7: Running job estimation display**
  - **Validates: Requirements 2.3**

- [x] 4. Improve detail view timing information
  - Ensure all timing fields are displayed for completed jobs
  - Add "(not started)" indicator for pending jobs
  - Add "(not finished)" indicator for running jobs
  - Ensure duration formatting is consistent (Xh Ym Zs format)
  - Add queue time, processing time, and total time calculations
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4.1 Write property test for completed job timing
  - **Property 9: Completed job timing completeness**
  - **Validates: Requirements 3.1**

- [x] 4.2 Write property test for pending job timing
  - **Property 10: Pending job timing display**
  - **Validates: Requirements 3.2**

- [x] 4.3 Write property test for duration formatting
  - **Property 12: Duration formatting consistency**
  - **Validates: Requirements 3.4**

- [x] 5. Enhance savings estimation display
  - Ensure estimates are calculated when all metadata is available
  - Add specific missing field indicators when metadata is incomplete
  - Ensure actual savings are used for completed jobs
  - Add "~" prefix for estimated values
  - Use quality-based estimation when av1_quality is set
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 5.1 Write property test for savings estimation
  - **Property 14: Savings estimation with complete metadata**
  - **Validates: Requirements 4.1**

- [x] 5.2 Write property test for missing field indication
  - **Property 15: Missing metadata field indication**
  - **Validates: Requirements 4.2**

- [x] 5.3 Write property test for actual savings preference
  - **Property 16: Actual savings preference**
  - **Validates: Requirements 4.3**

- [x] 5.4 Write property test for estimate prefix
  - **Property 18: Estimate prefix consistency**
  - **Validates: Requirements 4.5**

- [x] 6. Improve statistics dashboard
  - Ensure all statistics are displayed (space saved, compression ratio, processing time, success rate)
  - Add estimated pending savings calculation
  - Display zero values when no completed jobs exist
  - Ensure trend sparklines use last 20 jobs
  - Verify statistics refresh every 5 seconds
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 6.1 Write property test for statistics completeness
  - **Property 19: Statistics dashboard completeness**
  - **Validates: Requirements 5.1**

- [x] 6.2 Write property test for pending savings
  - **Property 20: Pending savings inclusion**
  - **Validates: Requirements 5.2**

- [x] 6.3 Write property test for zero statistics
  - **Property 21: Zero statistics display**
  - **Validates: Requirements 5.3**

- [x] 7. Enhance detail view completeness
  - Ensure all sections are displayed (file paths, status, job history, video metadata, encoding parameters, file sizes)
  - Add "(not available)" / "(not set)" indicators for missing fields
  - Display encoding parameters when set (quality, profile, web-like flag)
  - Display file sizes in both human-readable and exact byte formats
  - Ensure compression calculations are accurate
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.6_

- [x] 7.1 Write property test for detail view completeness
  - **Property 3: Detail view metadata completeness**
  - **Validates: Requirements 1.3, 6.1, 6.6**

- [x] 7.2 Write property test for encoding parameters display
  - **Property 23: Encoding parameters display**
  - **Validates: Requirements 6.2**

- [x] 7.3 Write property test for file size display
  - **Property 24: Dual format file size display**
  - **Validates: Requirements 6.3**

- [x] 7.4 Write property test for compression calculation
  - **Property 25: Compression calculation accuracy**
  - **Validates: Requirements 6.4**

- [x] 8. Verify responsive layout behavior
  - Test narrow terminal column visibility (< 80 columns)
  - Test short terminal component hiding (< 20 lines)
  - Test very small terminal simplified view (< 80x12)
  - Verify column priority in constrained layouts
  - Verify layout recalculation on resize
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 8.1 Write property test for narrow terminal layout
  - **Property 26: Narrow terminal column visibility**
  - **Validates: Requirements 7.1**

- [x] 8.2 Write property test for short terminal layout
  - **Property 27: Short terminal component visibility**
  - **Validates: Requirements 7.2**

- [x] 8.3 Write property test for very small terminal
  - **Property 28: Very small terminal simplified view**
  - **Validates: Requirements 7.3**

- [x] 8.4 Write property test for column priority
  - **Property 29: Column priority in constrained layouts**
  - **Validates: Requirements 7.5**

- [x] 9. Verify system resource display
  - Ensure activity status is displayed with correct icon
  - Ensure CPU usage is displayed
  - Ensure memory usage is displayed in GB and percentage
  - Verify "Idle" status when no jobs running
  - Verify "Processing" status when jobs running
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 9.1 Write property test for activity status
  - **Property 30: Activity status display accuracy**
  - **Validates: Requirements 8.1, 8.4, 8.5**

- [x] 10. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
