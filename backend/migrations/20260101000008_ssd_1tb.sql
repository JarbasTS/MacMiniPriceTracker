-- SSD M.2 1TB (NVMe ou SATA M.2), qualquer marca. normalize() transforma "M.2" em "m 2" e "1 TB" em "1 tb".
-- Exclui máquinas inteiras (notebook, PC) que citam o SSD no título; teto de R$ 2000 como segunda barreira.
-- A Apple Store já só busca chips de Mac (migration 7); as demais lojas buscam todos os alvos.
INSERT INTO search_targets (name, chip_family, search_query, required_keywords, excluded_keywords, min_price_brl, max_price_brl) VALUES
('SSD M.2 1TB', 'SSD 1TB', 'ssd m.2 1tb', ARRAY['ssd', 'm 2|m2|nvme', '1tb|1 tb|1000gb|1000 gb'],
 ARRAY['case', 'gaveta', 'adaptador', 'enclosure', 'dissipador', 'heatsink', 'externo', 'portatil', 'usb',
       'kit', 'defeito', 'usado', '2tb', '2 tb', '4tb', '4 tb', '512gb', '500gb', '256gb',
       'notebook', 'macbook', 'pc', 'computador', 'gamer', 'desktop'], 250.00, 2000.00);
