//! Outbound-port adapters that sit on top of the usecase.
//! Today only the in-memory `CrfServiceImpl` (which adapts
//! `CrfUsecase` to `apis::crf::CrfService`) lives here.

pub mod in_memory;
