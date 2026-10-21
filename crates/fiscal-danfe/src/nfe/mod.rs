//! The DANFE of the NF-e (model 55): report assembly and rendering.

mod billing;
mod footer;
mod header;
mod items;
mod parties;
mod totals;

use std::sync::Arc;

use fiscal_pdf::report::{Band, BandType, Engine, Report};
use fiscal_pdf::{Orientation, PageFormat, Unit};

use crate::context::DanfeContext;
use crate::error::DanfeError;
use crate::model::{Nfe, TpImp};
use crate::options::{DanfeOptions, InformacoesDePagamento};

use header::{Canhoto, DEFAULT_FONT_FAMILY, DadosNfe, LogoAlign};

/// Adds the pages and bands of the DANFE to `report` (`OnStartReport` of the original).
fn build_report(report: &mut Report, ctx: &Arc<DanfeContext>) {
    let options = &ctx.options;
    let orientation = if ctx.nfe.ide.tp_imp == TpImp::Paisagem {
        Orientation::Landscape
    } else {
        Orientation::Portrait
    };
    let (left, top, right, bottom) = options.effective_margins();
    report.set_margins(left, top, Some(right), Some(bottom));

    let logo = options.logo.clone().unwrap_or_default();
    let logo_align = if options.logo_em_cima {
        LogoAlign::Center
    } else if options.expande_logo_marca {
        LogoAlign::Full
    } else {
        LogoAlign::Right
    };

    report.add_page(orientation, Unit::Mm, PageFormat::A4);
    let add = |report: &mut Report, band: Band| {
        // The report has exactly one page at this point.
        let _ = report.add_band(band);
    };
    add(
        report,
        Band::new(
            BandType::Watermark,
            footer::MarcaDagua {
                ctx: Arc::clone(ctx),
                cancelada: options.cancelada,
            },
        ),
    );
    let canhoto_type = match options.pos_canhoto {
        crate::options::PosCanhoto::Cabecalho | crate::options::PosCanhoto::Nenhum => {
            BandType::TopMargin
        }
        crate::options::PosCanhoto::Esquerda => BandType::LeftMargin,
        crate::options::PosCanhoto::Rodape => BandType::BottomMargin,
    };
    add(
        report,
        Band::new(
            canhoto_type,
            Canhoto {
                ctx: Arc::clone(ctx),
                align: options.pos_canhoto,
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::PageHeader,
            DadosNfe {
                ctx: Arc::clone(ctx),
                logo,
                logo_stretched: false,
                logo_align,
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::Data,
            parties::Destinatario {
                ctx: Arc::clone(ctx),
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::Data,
            parties::LocalBand {
                ctx: Arc::clone(ctx),
                kind: parties::LocalKind::Retirada,
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::Data,
            parties::LocalBand {
                ctx: Arc::clone(ctx),
                kind: parties::LocalKind::Entrega,
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::Data,
            billing::FaturaDuplicatas::new(Arc::clone(ctx)),
        ),
    );
    if options.exibe_campo_de_pagamento == InformacoesDePagamento::Quadro {
        add(
            report,
            Band::new(
                BandType::Data,
                billing::Pagamentos {
                    ctx: Arc::clone(ctx),
                },
            ),
        );
    }
    add(
        report,
        Band::new(
            BandType::Data,
            totals::CalculoImposto {
                ctx: Arc::clone(ctx),
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::Data,
            totals::Transporte {
                ctx: Arc::clone(ctx),
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::Data,
            items::ProdutosServicos::new(Arc::clone(ctx)),
        ),
    );
    add(
        report,
        Band::new(
            BandType::Data,
            totals::CalculoIssqn {
                ctx: Arc::clone(ctx),
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::PageFooter,
            footer::DadosAdicionais {
                ctx: Arc::clone(ctx),
            },
        ),
    );
    add(
        report,
        Band::new(
            BandType::PageFooter,
            footer::Rodape {
                mensagem: options.sistema.clone(),
            },
        ),
    );
}

/// Renders the DANFE of an already parsed NF-e.
pub fn render_nfe(nfe: Nfe, options: &DanfeOptions) -> Result<Vec<u8>, DanfeError> {
    // Consumer invoices (model 65) use the receipt layout.
    if nfe.ide.modelo == 65 {
        return crate::nfce::render_nfce(nfe, options);
    }
    let compress = options.comprimir_pdf;
    let ctx = Arc::new(DanfeContext::new(nfe, options.clone()));

    let mut report = Report::new();
    report.set_font(DEFAULT_FONT_FAMILY);
    report.engine_options.double_pass = true;
    let mut initialised = false;
    report.set_on_start(move |report, _| {
        if !initialised {
            build_report(report, &ctx);
            initialised = true;
        }
    });

    let mut engine = Engine::new(&mut report);
    engine.set_compressed(compress);
    Ok(engine.render()?)
}

/// Renders the DANFE of an NF-e given as XML text (`nfeProc` or bare `NFe`).
pub fn render_danfe(xml: &str, options: &DanfeOptions) -> Result<Vec<u8>, DanfeError> {
    render_nfe(Nfe::from_xml(xml)?, options)
}

/// Renders the DANFE of an NF-e given as XML bytes (UTF-8 or Windows-1252).
pub fn render_danfe_bytes(xml: &[u8], options: &DanfeOptions) -> Result<Vec<u8>, DanfeError> {
    render_nfe(Nfe::from_xml_bytes(xml)?, options)
}
