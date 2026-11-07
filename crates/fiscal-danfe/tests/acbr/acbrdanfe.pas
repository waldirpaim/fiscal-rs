{ Harness do teste diferencial do fiscal-danfe contra o ACBr de verdade.

  Programa de console que imprime o DANFE (NF-e ou NFC-e, pelo modelo) ou o DANFE de um evento
  com o componente do ACBr, do mesmo jeito que o MCPServer o usa (Web.Controles.DanfePdf.Service):

    acbrdanfe nota.xml saida.pdf [Nome=Valor ...]

  Cada `Nome=Valor` é uma propriedade do componente (os nomes do ACBr: Sistema, MargemEsquerda,
  ExibeCampoDePagamento=eipQuadro...). Três nomes são tratados à parte porque o ACBr não os
  repassa ao relatório pelo componente: Cancelada, PosCanhoto (NF-e) e os `Exibir*` da NFC-e. O
  nome `Evento=arquivo.xml` imprime o evento (procEventoNFe) da nota.

  Este arquivo não contém código do ACBr: só usa as unidades dele, compiladas como caixa-preta
  por build.py. Ver README.md. }
program acbrdanfe;
{$I ACBr.inc}
uses
  Interfaces, SysUtils, Classes, Math, TypInfo,
  ACBrNFe, ACBrNFe.Classes, ACBrDFe.Conversao, ACBrDFeDANFeReport,
  ACBr_fpdf_report, ACBrNFeUtilsFPDF, ACBrNFeDANFEClass, ACBrNFeDANFeFPDF, ACBrNFCeDANFeFPDF;

var
  LXml, LOut, LModelo: string;
  LACBr: TACBrNFe;
  LReport: TACBrDFeDANFeReport;
  LStream: TMemoryStream;
  LNFe: TNFe;
  I, LPos: Integer;
  LCanhoto: ACBrNFeUtilsFPDF.TPosRecibo;
  LDireto: Boolean;
  LInner: TNFeDANFeFPDF;
  LEngine: TFPDFEngine;
  LInnerC: TNFCeDANFeFPDF;
  LEventoFile: string;
  LDiretoC: Boolean;
  LExibirItens, LExibirNomeFantasia, LExibirConsumidorNome, LExibirConsumidorEndereco: Boolean;
  LPair, LName, LValue: string;
begin
  // O Delphi do MCPServer usa strings ANSI em cp1252.
  SetMultiByteConversionCodePage(1252);
  DefaultSystemCodePage := 1252;
  LDireto := False;
  LEventoFile := '';
  LDiretoC := False;
  LExibirItens := True;
  LExibirNomeFantasia := False;
  LExibirConsumidorNome := False;
  LExibirConsumidorEndereco := False;
  if GetEnvironmentVariable('MASKFP') <> '' then
    SetExceptionMask(GetExceptionMask + [exInvalidOp, exZeroDivide, exOverflow]);
  // Como o Windows pt-BR em que o MCPServer roda.
  DefaultFormatSettings.DecimalSeparator := ',';
  DefaultFormatSettings.ThousandSeparator := '.';
  DefaultFormatSettings.DateSeparator := '/';
  DefaultFormatSettings.TimeSeparator := ':';
  DefaultFormatSettings.ShortDateFormat := 'dd/mm/yyyy';
  DefaultFormatSettings.LongDateFormat := 'dddd, d" de "mmmm" de "yyyy';
  DefaultFormatSettings.ShortTimeFormat := 'hh:nn';
  DefaultFormatSettings.LongTimeFormat := 'hh:nn:ss';
  if ParamCount < 2 then
  begin
    WriteLn('uso: acbrdanfe nota.xml saida.pdf [sistema]');
    Halt(2);
  end;
  LXml := ParamStr(1);
  LOut := ParamStr(2);
  LACBr := TACBrNFe.Create(nil);
  LStream := TMemoryStream.Create;
  try
    if not LACBr.NotasFiscais.LoadFromFile(LXml) or (LACBr.NotasFiscais.Count = 0) then
      raise Exception.Create('XML nao pode ser lido');
    LNFe := LACBr.NotasFiscais.Items[0].NFe;
    LModelo := IntToStr(LNFe.Ide.modelo);
    if LModelo = '65' then
      LReport := TACBrNFCeDANFeFPDF.Create(nil)
    else
      LReport := TACBrNFeDANFeFPDF.Create(nil);
    try
      LReport.NumCopias := 1;
      LReport.MostraPreview := False;
      LReport.MostraStatus := False;
      // Só o que vem na linha de comando (Nome=Valor, os nomes do ACBr): os padrões são os dele.
      for I := 3 to ParamCount do
      begin
        LPair := ParamStr(I);
        LPos := Pos('=', LPair);
        LName := Copy(LPair, 1, LPos - 1);
        LValue := Copy(LPair, LPos + 1, MaxInt);
        if SameText(LName, 'Cancelada') then
          LReport.Cancelada := SameText(LValue, 'True')
        else if SameText(LName, 'PosCanhoto') then
        begin
          // O componente nao repassa o PosCanhoto ao relatorio (a linha esta comentada no
          // ImprimirDANFEPDF): o relatorio interno e montado aqui.
          LDireto := True;
          LCanhoto := ACBrNFeUtilsFPDF.TPosRecibo(GetEnumValue(TypeInfo(ACBrNFeUtilsFPDF.TPosRecibo), LValue));
        end
        else if SameText(LName, 'Evento') then
          LEventoFile := LValue
        else if SameText(LName, 'ExibirItens') then begin LDiretoC := True; LExibirItens := SameText(LValue, 'True'); end
        else if SameText(LName, 'ExibirNomeFantasia') then begin LDiretoC := True; LExibirNomeFantasia := SameText(LValue, 'True'); end
        else if SameText(LName, 'ExibirConsumidorNome') then begin LDiretoC := True; LExibirConsumidorNome := SameText(LValue, 'True'); end
        else if SameText(LName, 'ExibirConsumidorEndereco') then begin LDiretoC := True; LExibirConsumidorEndereco := SameText(LValue, 'True'); end
        else if GetPropInfo(LReport, LName) = nil then
          WriteLn(StdErr, 'propriedade desconhecida: ', LName)
        else
          SetPropValue(LReport, LName, LValue);
      end;
      LReport.NomeDocumento := 'danfe.pdf';
      LACBr.DANFE := LReport;
      if LEventoFile <> '' then
      begin
        LACBr.EventoNFe.Evento.Clear;
        if not LACBr.EventoNFe.LerXML(LEventoFile) then
          raise Exception.Create('evento nao pode ser lido');
        LReport.ImprimirEVENTOPDF(LStream, LNFe);
      end
      else if LDireto and (LModelo <> '65') then
      begin
        LInner := TNFeDANFeFPDF.Create(LNFe, TACBrNFeDANFEClass(LReport));
        try
          LInner.PosCanhoto := LCanhoto;
          LInner.MensagemRodape := LReport.Sistema;
          LEngine := TFPDFEngine.Create(LInner, False);
          try
            LEngine.Compressed := True;
            LEngine.SaveToStream(LStream);
          finally
            LEngine.Free;
          end;
        finally
          LInner.Free;
        end;
      end
      else if LDiretoC and (LModelo = '65') then
      begin
        LInnerC := TNFCeDANFeFPDF.Create(LNFe, TACBrNFeDANFCEClass(LReport));
        try
          LInnerC.ExibirItens := LExibirItens;
          LInnerC.ExibirNomeFantasia := LExibirNomeFantasia;
          LInnerC.ExibirConsumidorNome := LExibirConsumidorNome;
          LInnerC.ExibirConsumidorEndereco := LExibirConsumidorEndereco;
          LInnerC.MensagemRodape := LReport.Sistema;
          LEngine := TFPDFEngine.Create(LInnerC, False);
          try
            LEngine.Compressed := True;
            LEngine.SaveToStream(LStream);
          finally
            LEngine.Free;
          end;
        finally
          LInnerC.Free;
        end;
      end
      else
        LReport.ImprimirDANFEPDF(LStream, LNFe);
      LStream.SaveToFile(LOut);
      WriteLn('ok ', LStream.Size, ' bytes');
    finally
      LACBr.DANFE := nil;
      LReport.Free;
    end;
  finally
    LStream.Free;
    LACBr.Free;
  end;
end.
