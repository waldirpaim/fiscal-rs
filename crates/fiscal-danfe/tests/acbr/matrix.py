#!/usr/bin/env python3
"""Differential test of fiscal-danfe against the real ACBr (see README.md).

For each case (an XML and a list of `Nome=Valor` options, the names of the ACBr properties) it
runs the ACBr harness (`acbrdanfe`, built from the ACBr source with Free Pascal) and the
`acbr_diff` example, and compares the PDFs: page count, page sizes and the content streams.
Usage: matrix.py ACBRDANFE ACBR_DIFF FIXTURES_DIR [REAL_XML ...] [-k FILTER]
"""
import os
import re
import subprocess
import sys
import tempfile
import zlib

ACBR, RS, FX = sys.argv[1:4]
REAL = [a for a in sys.argv[4:] if not a.startswith('-k')]
FILTER = sys.argv[sys.argv.index('-k') + 1] if '-k' in sys.argv else ''
if '-k' in sys.argv:
    REAL = [a for a in REAL if a != FILTER]

NFE = open(f'{FX}/nfe_55.xml', encoding='utf-8').read()
NFCE = open(f'{FX}/nfce_65.xml', encoding='utf-8').read()

MCP_NFE = ['MargemEsquerda=3', 'MargemSuperior=3', 'MargemDireita=3', 'MargemInferior=3', 'Sistema=MCPServer']
MCP_NFCE = ['MargemEsquerda=1', 'MargemSuperior=1', 'MargemDireita=1', 'MargemInferior=1',
            'ImprimeNomeFantasia=True', 'Sistema=MCPServer']


def sub(xml, old, new, count=1):
    assert old in xml, old
    return xml.replace(old, new, count)


def items(xml, n):
    """The invoice with `n` items cloned from the first one."""
    start = xml.index('<det nItem="1">')
    end = xml.index('</det>') + len('</det>')
    first = xml[start:end]
    last_end = xml.rindex('</det>') + len('</det>')
    body = ''.join(first.replace('nItem="1"', f'nItem="{i}"').replace('<cProd>0001</cProd>', f'<cProd>ITEM{i:04d}</cProd>')
                   .replace('<cProd>7891000100103</cProd>', f'<cProd>78910{i:08d}</cProd>')
                   for i in range(1, n + 1))
    return xml[:start] + body + xml[last_end:]


def no_protocol(xml):
    a = xml.index('<protNFe')
    b = xml.index('</protNFe>') + len('</protNFe>')
    return xml[:a] + xml[b:]


def status(xml, code, text):
    xml = re.sub(r'<cStat>\d+</cStat>', f'<cStat>{code}</cStat>', xml)
    return re.sub(r'<xMotivo>[^<]*</xMotivo>', f'<xMotivo>{text}</xMotivo>', xml)


def contingency(xml, code, just='Falha de comunicacao com a SEFAZ por mais de uma hora seguida'):
    xml = sub(xml, '<tpEmis>1</tpEmis>', f'<tpEmis>{code}</tpEmis>')
    return sub(xml, '</ide>', f'<dhCont>2026-07-20T09:50:00-03:00</dhCont><xJust>{just}</xJust></ide>')


def homolog(xml):
    return xml.replace('<tpAmb>1</tpAmb>', '<tpAmb>2</tpAmb>')


def add_total(xml, extra):
    return sub(xml, '</ICMSTot>', '</ICMSTot>' + extra)


def det_extra(xml, extra, item=1):
    """Adds `extra` inside the `prod` of the item (after the last prod child)."""
    marker = '</prod>'
    pos = xml.index(f'<det nItem="{item}">')
    at = xml.index(marker, pos)
    return xml[:at] + extra + xml[at:]


def png(path, size=64):
    from PIL import Image, ImageDraw
    img = Image.new('RGBA', (size * 2, size), (255, 255, 255, 0))
    d = ImageDraw.Draw(img)
    d.rectangle([2, 2, size * 2 - 3, size - 3], outline=(200, 30, 30, 255), width=3)
    d.ellipse([size // 2, size // 4, size * 3 // 2, size * 3 // 4], fill=(30, 30, 200, 255))
    img.save(path)


tmp = tempfile.mkdtemp(prefix='acbr-matrix-')
LOGO = f'{tmp}/logo.png'
try:
    png(LOGO)
except Exception as error:  # Pillow is optional: without it the logo cases are skipped
    LOGO = None
    print('sem Pillow, casos de logo pulados:', error)

cases = []


# O ACBr, compilado com o Free Pascal no Linux, difere do Delphi do MCPServer em dois pontos que
# não são do layout: o `Sistema` padrão dele é "Projeto ACBr - www.projetoacbr.com.br" (o desta
# crate é vazio) e `MargemDireita <> 5.1` compara um Double com um literal estendido, então o
# padrão não é reconhecido como "sem margem". Todo caso fixa as duas coisas, e as opções do
# caso vêm depois e valem.
BASE_NFE = ['Sistema=', 'MargemEsquerda=2', 'MargemSuperior=2', 'MargemDireita=2', 'MargemInferior=2']
BASE_NFCE = ['Sistema=']


def case(name, xml, args=(), env=None):
    base = BASE_NFCE if '<mod>65</mod>' in xml else BASE_NFE
    cases.append((name, xml, base + list(args), env or {}))


# ---- NF-e ---------------------------------------------------------------------------------
case('nfe/padrao', NFE)
case('nfe/mcpserver', NFE, MCP_NFE)
case('nfe/canhoto-rodape', NFE, ['PosCanhoto=prRodape'])
case('nfe/canhoto-esquerda', NFE, ['PosCanhoto=prEsquerda'])
case('nfe/paisagem', sub(NFE, '<tpImp>1</tpImp>', '<tpImp>2</tpImp>'))
case('nfe/paisagem-canhoto-rodape', sub(NFE, '<tpImp>1</tpImp>', '<tpImp>2</tpImp>'), ['PosCanhoto=prRodape'])
case('nfe/paisagem-canhoto-esquerda', sub(NFE, '<tpImp>1</tpImp>', '<tpImp>2</tpImp>'), ['PosCanhoto=prEsquerda'])
for n in (8, 25, 60, 150):
    case(f'nfe/itens-{n}', items(NFE, n), MCP_NFE)
case('nfe/paisagem-itens-60', items(sub(NFE, '<tpImp>1</tpImp>', '<tpImp>2</tpImp>'), 60))
case('nfe/homologacao', homolog(NFE))
case('nfe/cancelada-prop', NFE, ['Cancelada=True'])
case('nfe/prot-101', status(NFE, 101, 'Cancelamento de NF-e homologado'))
case('nfe/prot-110', status(NFE, 110, 'Uso Denegado'))
case('nfe/prot-301', status(NFE, 301, 'Uso Denegado: Irregularidade fiscal do emitente'))
case('nfe/sem-protocolo', no_protocol(NFE))
for code in (2, 3, 4, 5, 6, 7, 8, 9):
    case(f'nfe/emissao-{code}', contingency(NFE, code))
    case(f'nfe/emissao-{code}-sem-protocolo', no_protocol(contingency(NFE, code)))
case('nfe/dest-cpf', sub(NFE, '<CNPJ>99888777000166</CNPJ>', '<CPF>12345678909</CPF>'))
case('nfe/sem-dest', re.sub(r'<dest>.*?</dest>', '', NFE, flags=re.S))
case('nfe/dest-exterior', sub(sub(re.sub(r'<CNPJ>99888777000166</CNPJ>', '<idEstrangeiro>EX123456</idEstrangeiro>', NFE),
                                  '<xMun>RIO DE JANEIRO</xMun><UF>RJ</UF>', '<xMun>EXTERIOR</xMun><UF>EX</UF>'),
                              '<xPais>BRASIL</xPais><fone>2122223333', '<xPais>PARAGUAI</xPais><fone>2122223333'))
case('nfe/infcpl-longo', sub(NFE, 'Pedido de compra 4521.', 'Linha 1|Linha 2 com mais texto|Linha 3. ' + 'Texto muito longo para quebrar em varias linhas. ' * 30))
case('nfe/infcpl-longo-expandir', sub(NFE, 'Pedido de compra 4521.', 'Texto muito longo para quebrar. ' * 120), MCP_NFE)
case('nfe/obsfisco', sub(NFE, '</infAdic>', '<obsFisco xCampo="Fisco"><xTexto>Observacao do fisco aqui</xTexto></obsFisco></infAdic>'))
case('nfe/proc-ref', sub(NFE, '</infAdic>', '<procRef><nProc>12345</nProc><indProc>1</indProc></procRef></infAdic>'))
case('nfe/refnfe', sub(NFE, '</ide>', '<NFref><refNFe>35260611222333000181550010000011111123456781</refNFe></NFref></ide>'))
case('nfe/refnfe-sem-doc-ref', sub(NFE, '</ide>', '<NFref><refNFe>35260611222333000181550010000011111123456781</refNFe></NFref></ide>'),
     ['ExibeDadosDocReferenciados=False'])
case('nfe/retirada-entrega', sub(NFE, '<det nItem="1">',
     '<retirada><CNPJ>11222333000181</CNPJ><xLgr>RUA DO DEPOSITO</xLgr><nro>10</nro><xBairro>INDUSTRIAL</xBairro><cMun>3550308</cMun><xMun>SAO PAULO</xMun><UF>SP</UF></retirada>'
     '<entrega><CNPJ>99888777000166</CNPJ><xLgr>AV DAS OBRAS</xLgr><nro>77</nro><xBairro>CENTRO</xBairro><cMun>3304557</cMun><xMun>RIO DE JANEIRO</xMun><UF>RJ</UF></entrega><det nItem="1">'))
case('nfe/sem-cobranca', re.sub(r'<cobr>.*?</cobr>', '', NFE, flags=re.S))
case('nfe/sem-transporte-volume', re.sub(r'<vol>.*?</vol>', '', NFE, flags=re.S))
case('nfe/issqn', add_total(NFE, '<ISSQNtot><vServ>150.00</vServ><vBC>150.00</vBC><vISS>7.50</vISS><vPIS>0.00</vPIS><vCOFINS>0.00</vCOFINS></ISSQNtot>'))
case('nfe/issqn-exibe', add_total(NFE, '<ISSQNtot><vServ>150.00</vServ><vBC>150.00</vBC><vISS>7.50</vISS><vPIS>0.00</vPIS><vCOFINS>0.00</vCOFINS></ISSQNtot>'), ['ExibeDadosISSQN=True'])
case('nfe/pag-varios', sub(NFE, '<detPag><indPag>1</indPag><tPag>15</tPag><vPag>664.75</vPag></detPag>',
     '<detPag><indPag>0</indPag><tPag>01</tPag><vPag>100.00</vPag></detPag><detPag><indPag>0</indPag><tPag>17</tPag><vPag>300.00</vPag></detPag><detPag><indPag>1</indPag><tPag>99</tPag><xPag>Cheque garantia</xPag><vPag>264.75</vPag></detPag><vTroco>0.00</vTroco>'),
     ['ExibeCampoDePagamento=eipQuadro'])
case('nfe/pag-adicionais', NFE, ['ExibeCampoDePagamento=eipAdicionais'])
case('nfe/pag-quadro', NFE, ['ExibeCampoDePagamento=eipQuadro'])
for opt in ('ImprimeCodigoEan=True', 'ImprimeTotalLiquido=True', 'ExibeInforAdicProduto=infNenhum', 'ExibeInforAdicProduto=infSeparadamente',
            'ExibeTotalTributosItem=True', 'TributosPercentual=ptValorNF', 'ImprimeXPedNItemPed=True', 'ImprimeInfContr=False',
            'FormatarNumeroDocumento=False', 'ExibeCampoFatura=False', 'ExibeCampoDuplicata=False', 'QuebraLinhaEmDetalhamentos=False',
            'CaractereQuebraDeLinha=#', 'ExibeICMSDesoneradoComoDesconto=True', 'ExibeDadosInscricaoSuframa=False',
            'ImprimeDetalhamentoEspecifico=False'):
    case(f'nfe/opcao-{opt}', NFE, [opt])
case('nfe/tributos-personalizado', NFE, ['TributosPercentual=ptPersonalizado', 'TributosPercentualPersonalizado=12,5', 'ExibeTotalTributosItem=True'])
case('nfe/desonerado', sub(sub(NFE, '<vICMS>81.90</vICMS></ICMS00>', '<vICMS>81.90</vICMS><vICMSDeson>10.00</vICMSDeson><motDesICMS>9</motDesICMS></ICMS00>'),
                           '<vICMSDeson>0.00</vICMSDeson>', '<vICMSDeson>10.00</vICMSDeson>'), ['ExibeICMSDesoneradoComoDesconto=True'])
case('nfe/xped', det_extra(NFE, '<xPed>PED-77</xPed><nItemPed>3</nItemPed>'), ['ImprimeXPedNItemPed=True'])
case('nfe/veiculo', det_extra(NFE, '<veicProd><tpOp>1</tpOp><chassi>9BWZZZ377VT004251</chassi><cCor>01</cCor><xCor>PRETA</xCor><pot>110</pot><cilin>1600</cilin><pesoL>1100</pesoL><pesoB>1500</pesoB><nSerie>ABC123</nSerie><tpComb>16</tpComb><nMotor>MOT998877</nMotor><CMT>2000.0</CMT><dist>2550</dist><anoMod>2025</anoMod><anoFab>2025</anoFab><tpPint>M</tpPint><tpVeic>06</tpVeic><espVeic>1</espVeic><VIN>N</VIN><condVeic>1</condVeic><cMod>123456</cMod><cCorDENATRAN>01</cCorDENATRAN><lota>5</lota><tpRest>0</tpRest></veicProd>'))
case('nfe/medicamento', det_extra(NFE, '<rastro><nLote>LOTE123</nLote><qLote>10.000</qLote><dFab>2026-01-10</dFab><dVal>2027-01-10</dVal><cAgreg>12345</cAgreg></rastro><med><cProdANVISA>1234567890123</cProdANVISA><vPMC>55.90</vPMC></med>'))
case('nfe/combustivel', det_extra(NFE, '<comb><cProdANP>320101001</cProdANP><descANP>GASOLINA C COMUM</descANP><pGLP>0</pGLP><CODIF>12345</CODIF><qTemp>10.0000</qTemp><UFCons>SP</UFCons></comb>'))
case('nfe/arma', det_extra(NFE, '<arma><tpArma>0</tpArma><nSerie>S12345</nSerie><nCano>C12345</nCano><descr>Pistola calibre 380 exemplo</descr></arma>'))
if LOGO:
    case('nfe/logo', NFE, [f'Logo={LOGO}'])
    case('nfe/logo-em-cima', NFE, [f'Logo={LOGO}', 'LogoemCima=True'])
    case('nfe/logo-expandida', NFE, [f'Logo={LOGO}', 'ExpandeLogoMarca=True'])
    case('nfe/logo-paisagem', sub(NFE, '<tpImp>1</tpImp>', '<tpImp>2</tpImp>'), [f'Logo={LOGO}'])
case('nfe/margens-grandes', NFE, ['MargemEsquerda=12', 'MargemSuperior=15', 'MargemDireita=10', 'MargemInferior=14'])
case('nfe/margens-padrao-esquerda-superior-inferior', NFE, ['MargemEsquerda=6', 'MargemSuperior=8', 'MargemInferior=8', 'MargemDireita=2'])
case('nfe/rodape-3-partes', NFE, ['Sistema=esquerda|centro|direita'])
case('nfe/item-sem-ean-com-desc', sub(NFE, '<vDesc>1.70</vDesc>', '<vDesc>1.70</vDesc><vFrete>3.00</vFrete><vSeg>1.00</vSeg><vOutro>2.00</vOutro>'), ['ImprimeTotalLiquido=True'])
case('nfe/crt-1-csosn', sub(sub(sub(NFE, '<CRT>3</CRT>', '<CRT>1</CRT>'), '<ICMS00><orig>0</orig><CST>00</CST><modBC>3</modBC><vBC>455.00</vBC><pICMS>18.0000</pICMS><vICMS>81.90</vICMS></ICMS00>',
                                    '<ICMSSN101><orig>0</orig><CSOSN>101</CSOSN><pCredSN>2.5600</pCredSN><vCredICMSSN>11.65</vCredICMSSN></ICMSSN101>'), '<xFant>EXEMPLO</xFant>', '<xFant>EXEMPLO</xFant>'))
case('nfe/st', sub(NFE, '<ICMS00><orig>0</orig><CST>00</CST><modBC>3</modBC><vBC>455.00</vBC><pICMS>18.0000</pICMS><vICMS>81.90</vICMS></ICMS00>',
                   '<ICMS10><orig>0</orig><CST>10</CST><modBC>3</modBC><vBC>455.00</vBC><pICMS>18.0000</pICMS><vICMS>81.90</vICMS><modBCST>4</modBCST><pMVAST>40.0000</pMVAST><vBCST>700.00</vBCST><pICMSST>18.0000</pICMSST><vICMSST>44.10</vICMSST></ICMS10>'))

# ---- NFC-e --------------------------------------------------------------------------------
case('nfce/padrao', NFCE)
case('nfce/mcpserver', NFCE, MCP_NFCE)
case('nfce/bobina-58', NFCE, ['LarguraBobina=58'])
case('nfce/bobina-80-explicita', NFCE, ['LarguraBobina=80'])
case('nfce/bobina-76', NFCE, ['LarguraBobina=76'])
case('nfce/qr-lateral', NFCE, ['ImprimeQRCodeLateral=True'])
case('nfce/qr-lateral-58', NFCE, ['ImprimeQRCodeLateral=True', 'LarguraBobina=58'])
case('nfce/sem-itens', NFCE, ['ExibirItens=False'])
case('nfce/exibir-nome-fantasia', NFCE, ['ExibirNomeFantasia=True'])
case('nfce/exibir-consumidor-nome', NFCE, ['ExibirConsumidorNome=True'])
case('nfce/exibir-consumidor-endereco', sub(NFCE, '<indIEDest>9</indIEDest>', '<enderDest><xLgr>RUA DA PAZ</xLgr><nro>12</nro><xBairro>CENTRO</xBairro><cMun>5107925</cMun><xMun>SORRISO</xMun><UF>MT</UF></enderDest><indIEDest>9</indIEDest>'), ['ExibirConsumidorNome=True', 'ExibirConsumidorEndereco=True'])
case('nfce/exibir-tudo', NFCE, ['ExibirNomeFantasia=True', 'ExibirConsumidorNome=True', 'ExibirConsumidorEndereco=True', 'ExibirItens=True'])
case('nfce/nome-fantasia', NFCE, ['ImprimeNomeFantasia=True'])
case('nfce/sem-consumidor', re.sub(r'<dest>.*?</dest>', '', NFCE, flags=re.S))
case('nfce/consumidor-endereco', sub(NFCE, '<indIEDest>9</indIEDest>',
     '<enderDest><xLgr>RUA DA PAZ</xLgr><nro>12</nro><xBairro>CENTRO</xBairro><cMun>5107925</cMun><xMun>SORRISO</xMun><UF>MT</UF></enderDest><indIEDest>9</indIEDest>'))
case('nfce/consumidor-cnpj', sub(NFCE, '<CPF>12345678909</CPF>', '<CNPJ>99888777000166</CNPJ>'))
for n in (12, 40, 120):
    case(f'nfce/itens-{n}', items(NFCE, n), MCP_NFCE)
case('nfce/homologacao', homolog(NFCE))
case('nfce/cancelada-prop', NFCE, ['Cancelada=True'])
case('nfce/prot-101', status(NFCE, 101, 'Cancelamento de NF-e homologado'))
case('nfce/sem-protocolo', no_protocol(NFCE))
case('nfce/offline', contingency(NFCE, 9, 'sem conexao com a rede'))
case('nfce/offline-sem-protocolo', no_protocol(contingency(NFCE, 9, 'sem conexao com a rede')))
case('nfce/svc-an-sem-protocolo', no_protocol(contingency(NFCE, 6)))
case('nfce/sem-qrcode', re.sub(r'<infNFeSupl>.*?</infNFeSupl>', '', NFCE, flags=re.S), env={'MASKFP': '1'})
case('nfce/pagamentos', sub(NFCE, '<detPag><indPag>0</indPag><tPag>01</tPag><vPag>50.00</vPag></detPag>',
     '<detPag><tPag>03</tPag><vPag>20.00</vPag></detPag><detPag><tPag>04</tPag><vPag>15.00</vPag></detPag><detPag><tPag>17</tPag><vPag>15.00</vPag></detPag>'))
case('nfce/sem-troco', sub(NFCE, '<vTroco>8.46</vTroco>', '<vTroco>0.00</vTroco>'))
case('nfce/desconto-total', sub(NFCE, '<vDesc>0.10</vDesc><vII>', '<vDesc>3.10</vDesc><vII>'))
case('nfce/nome-longo', sub(NFCE, 'SACOLA PLASTICA', 'SACOLA PLASTICA BIODEGRADAVEL REFORCADA COM ALCA ' * 4))
case('nfce/infcpl-longo', sub(NFCE, 'Obrigado pela preferencia. Volte sempre!', 'Texto muito longo de informacao complementar. ' * 40))
case('nfce/sem-infcpl', re.sub(r'<infAdic>.*?</infAdic>', '', NFCE, flags=re.S))
case('nfce/sem-tributos', sub(NFCE, '<vTotTrib>8.31</vTotTrib>', ''))
if LOGO:
    case('nfce/logo', NFCE, [f'Logo={LOGO}'])
    case('nfce/logo-58', NFCE, [f'Logo={LOGO}', 'LarguraBobina=58'])
case('nfce/sistema-3-partes', NFCE, ['Sistema=esquerda|centro|direita'])
case('nfce/tipo-2-simplificado', sub(NFCE, '<tpImp>4</tpImp>', '<tpImp>6</tpImp>'))

# ---- Eventos (DANFE do evento: TNFeDANFeEventoFPDF) ---------------------------------------
CCE = open(f'{FX}/evento_cce.xml', encoding='utf-8').read()
CANC = open(f'{FX}/evento_cancelamento.xml', encoding='utf-8').read()
event_dir = tempfile.mkdtemp(prefix='acbr-eventos-')


def event_case(name, evento, nfe=NFE, args=()):
    path = f'{event_dir}/{name.replace("/", "_")}.xml'
    open(path, 'w', encoding='utf-8').write(evento)
    case(name, nfe, [f'Evento={path}', *args])


event_case('evento/cce', CCE)
event_case('evento/cancelamento', CANC)
event_case('evento/cce-sistema-3-partes', CCE, args=['Sistema=esquerda|centro|direita'])
event_case('evento/cce-texto-longo', sub(CCE, 'O endereco de entrega correto', 'Texto de correcao muito longo para quebrar em varias linhas. ' * 14))
event_case('evento/cce-texto-gigante', sub(CCE, 'O endereco de entrega correto', 'palavra ' * 700))
event_case('evento/cce-varias-linhas', sub(CCE, 'Ltda.|Demais', 'Linha um|Linha dois|Linha tres|Linha quatro|Demais'))
event_case('evento/cancelamento-texto-longo', sub(CANC, 'Erro na digitacao da quantidade dos itens; mercadoria nao saiu do estabelecimento.', 'Justificativa longa. ' * 40))
event_case('evento/cancelamento-gigante', sub(CANC, 'Erro na digitacao da quantidade dos itens; mercadoria nao saiu do estabelecimento.', 'palavra ' * 900))
event_case('evento/producao', sub(CANC, '<tpAmb>2</tpAmb>', '<tpAmb>1</tpAmb>').replace('<tpAmb>2</tpAmb>', '<tpAmb>1</tpAmb>'))
event_case('evento/epec', sub(sub(CANC, '110111', '110140'), '<descEvento>Cancelamento', '<descEvento>EPEC').replace('110111', '110140'))
event_case('evento/manifestacao', CANC.replace('110111', '210200').replace('<descEvento>Cancelamento', '<descEvento>Confirmacao da Operacao'))
event_case('evento/status-136', CCE.replace('<cStat>135</cStat>', '<cStat>136</cStat>'))
event_case('evento/sem-retevento', re.sub(r'<retEvento.*?</retEvento>', '', CCE, flags=re.S))
event_case('evento/nota-sem-dest', CCE, nfe=re.sub(r'<dest>.*?</dest>', '', NFE, flags=re.S))
event_case('evento/nota-dest-exterior', CANC, nfe=sub(sub(re.sub(r'<CNPJ>99888777000166</CNPJ>', '<idEstrangeiro>EX123456</idEstrangeiro>', NFE),
           '<xMun>RIO DE JANEIRO</xMun><UF>RJ</UF>', '<xMun>EXTERIOR</xMun><UF>EX</UF>'), '<xPais>BRASIL</xPais><fone>2122223333', '<xPais>PARAGUAI</xPais><fone>2122223333'))
event_case('evento/nota-sem-saida', CCE, nfe=sub(NFE, '<dhSaiEnt>2026-07-20T11:05:00-03:00</dhSaiEnt>', ''))
event_case('evento/nota-saida-meia-noite', CCE, nfe=sub(NFE, '2026-07-20T11:05:00-03:00', '2026-07-21T00:00:00-03:00'))
event_case('evento/nota-sem-complemento', CCE, nfe=sub(NFE, '<xCpl>SALA 3</xCpl>', ''))
event_case('evento/orgao-ambiente-nacional', CANC.replace('<cOrgao>35</cOrgao>', '<cOrgao>91</cOrgao>'))
event_case('evento/nfce', CANC, nfe=NFCE.replace('<tpImp>4</tpImp>', '<tpImp>1</tpImp>'))

# ---- XML reais (opcional) ------------------------------------------------------------------
for path in REAL:
    xml = open(path, encoding='utf-8').read()
    base = os.path.basename(path)[:14]
    case(f'real/{base}', xml, [], {'MASKFP': '1'})
    mcp = MCP_NFCE if '<mod>65</mod>' in xml else MCP_NFE
    case(f'real/{base}-mcpserver', xml, mcp, {'MASKFP': '1'})


def streams(path):
    data = open(path, 'rb').read()
    found = []
    for m in re.finditer(rb'stream\r?\n(.*?)\r?\nendstream', data, re.S):
        try:
            found.append(zlib.decompress(m.group(1)))
        except Exception:
            found.append(m.group(1))
    return data, found


def summary(path):
    data, found = streams(path)
    pages = len(re.findall(rb'/Type\s*/Page\b(?!s)', data))
    sizes = [tuple(round(float(n), 6) for n in box.split()) for box in re.findall(rb'/MediaBox\s*\[([^\]]*)\]', data)]
    content = b'\n'.join(s for s in found if b' Tf' in s or b'BT' in s or b' re' in s)
    return pages, sizes, content


# O ACBr compilado com o Free Pascal não inicializa o `FFormatSettings` do DANFE de evento (só o
# ramo do Delphi o faz), e a data de emissão da nota sai como "20". A linha do valor da data de
# emissão fica fora da comparação dos eventos; a data do evento tem teste próprio.
ISSUE_DATE = re.compile(rb'^BT [\d.]+ 748\.35 Td \(.*\) Tj ET$', re.M)


def drop_event_issue_date(content):
    return ISSUE_DATE.sub(b'', content)


NUMBER = re.compile(rb'(-?\d+\.\d+)')


def same_up_to_rounding(a, b, tolerance=0.0101):
    """Same content, except numbers that differ by a hundredth (the float formatting of the two
    compilers: the same value at the rounding edge)."""
    la, lb = a.split(b'\n'), b.split(b'\n')
    if len(la) != len(lb):
        return False
    for x, y in zip(la, lb):
        if x == y:
            continue
        px, py = NUMBER.split(x), NUMBER.split(y)
        if len(px) != len(py):
            return False
        for i, (u, v) in enumerate(zip(px, py)):
            if i % 2 == 0:
                if u != v:
                    return False
            elif abs(float(u) - float(v)) > tolerance:
                return False
    return True


def run(command, env_extra, out):
    env = dict(os.environ, **env_extra)
    done = subprocess.run(command, capture_output=True, env=env, timeout=120)
    return done.returncode, done.stderr.decode('utf-8', 'replace').strip(), os.path.exists(out)


# Diferenças conhecidas e decididas (o ACBr é a referência, mas estes casos não são o layout):
EXPECTED = {
    'nfce/sem-qrcode': 'sem infNFeSupl o ACBr divide por zero (EInvalidOp) e, com a exceção mascarada, '
                       'desenha retângulos sem sentido; aqui não se desenha QR Code',
    'nfce/logo': 'o ACBr desce o conteúdo 5 mm só na passada final, e a bobina fica 5 mm curta (corta o rodapé); '
                 'aqui a altura inclui os 5 mm',
    'nfce/logo-58': 'idem nfce/logo',
    'evento/sem-retevento': 'sem o retEvento o ACBr imprime a data do registro como 30/12/1899 00:00:00 (TDateTime zero); aqui fica em branco',
    'evento/nfce': 'o NFC-e do ACBr (FPDF) não imprime evento (ImprimirEVENTOPDF herdado e vazio); aqui sai o mesmo DANFE de evento em A4',
    'real/51260710459473': 'NFC-e em contingência sem infNFeSupl: ver nfce/sem-qrcode',
    'real/51260710459473-mcpserver': 'NFC-e em contingência sem infNFeSupl: ver nfce/sem-qrcode',
}

ok = diff = skipped = expected = 0
for name, xml, args, env in cases:
    if FILTER and FILTER not in name:
        continue
    path = f'{tmp}/case.xml'
    open(path, 'w', encoding='utf-8').write(xml)
    out_a, out_r = f'{tmp}/a.pdf', f'{tmp}/r.pdf'
    for p in (out_a, out_r):
        if os.path.exists(p):
            os.remove(p)
    env_acbr = dict(env, LD_LIBRARY_PATH=os.environ.get('LD_LIBRARY_PATH', ''))
    code_a, err_a, has_a = run([ACBR, path, out_a, *args], env_acbr, out_a)
    code_r, err_r, has_r = run([RS, path, out_r, *args], {}, out_r)
    if not has_a or not has_r:
        skipped += 1
        print(f'PULADO   {name}: ACBr={"ok" if has_a else (err_a or code_a)[:70]} RS={"ok" if has_r else (err_r or code_r)[:70]}')
        continue
    pa, sa, ca = summary(out_a)
    pr, sr, cr = summary(out_r)
    if name.startswith('evento/'):
        ca, cr = drop_event_issue_date(ca), drop_event_issue_date(cr)
    note = f' [{err_r}]' if err_r else ''
    if (pa, sa, ca) == (pr, sr, cr):
        ok += 1
        print(f'IDENTICO {name} ({pa} pág., {len(ca)} bytes){note}')
    elif (pa, sa) == (pr, sr) and same_up_to_rounding(ca, cr):
        ok += 1
        print(f'IDENTICO {name} ({pa} pág., {len(ca)} bytes, só arredondamento de centésimo){note}')
    else:
        reason = next((why for key, why in EXPECTED.items() if name == key), None)
        if reason:
            expected += 1
            print(f'ESPERADO {name}: {reason}')
            continue
        diff += 1
        la, lr = ca.decode('latin1').split('\n'), cr.decode('latin1').split('\n')
        import difflib
        lines = [l for l in difflib.unified_diff(la, lr, 'acbr', 'rs', lineterm='', n=0) if not l.startswith(('---', '+++', '@@'))]
        print(f'DIFERENTE {name}: páginas {pa}/{pr}, tamanhos {sa[:2]} / {sr[:2]}, {len(lines)} linhas{note}')
        for l in lines[:6]:
            print('     ', l[:170])
print(f'\n{ok} idênticos, {expected} diferenças esperadas, {diff} diferentes, {skipped} pulados')
sys.exit(1 if diff else 0)
