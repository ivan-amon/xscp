--
-- XSCP (eXtremely Simple Chat Protocol) Wireshark dissector
--
-- Text-based protocol over TCP (default port 7878). Fields are delimited by '|'
-- and every PDU ends with CRLF ("\r\n"). Three PDU kinds, told apart by the
-- first field:
--
--   Request      (client -> server):  OPCODE|Source|Message\r\n   OPCODE in {LOGN,SEND,EXIT}
--   Notification (server -> client):  BRDC|Source|Message\r\n
--   Response     (server -> client):  StatusCode|ReasonPhrase\r\n  StatusCode = 1-3 digits
--
-- "Message" may itself contain '|', so only the first two separators are
-- significant (mirrors splitn(3) in the Rust implementation).
--
-- Installation: copy this file into your Wireshark Lua plugins folder
--   (Help > About Wireshark > Folders > "Personal Lua Plugins"), or run
--   `wireshark -X lua_script:tools/wireshark/xscp.lua`. Reload with Ctrl+Shift+L.

local DEFAULT_PORT = 7878

local xscp = Proto("xscp", "XSCP Protocol")

-- Fields. Status code is kept as a (filterable) integer even though it travels
-- as ASCII text on the wire; everything else is a string.
local f_kind     = ProtoField.string("xscp.kind",            "Kind")
local f_opcode   = ProtoField.string("xscp.opcode",          "Opcode")
local f_ntype    = ProtoField.string("xscp.notification_type","Notification Type")
local f_status   = ProtoField.uint16("xscp.status_code",     "Status Code", base.DEC)
local f_reason   = ProtoField.string("xscp.reason_phrase",   "Reason Phrase")
local f_source   = ProtoField.string("xscp.source",          "Source")
local f_message  = ProtoField.string("xscp.message",         "Message")

xscp.fields = { f_kind, f_opcode, f_ntype, f_status, f_reason, f_source, f_message }

-- Expert info for things that don't conform to the spec.
local ef_malformed = ProtoExpert.new("xscp.malformed.expert", "Malformed XSCP PDU",
                                     expert.group.MALFORMED, expert.severity.WARN)
local ef_unknown   = ProtoExpert.new("xscp.unknown.expert", "Unknown XSCP first field",
                                     expert.group.UNDECODED, expert.severity.NOTE)
xscp.experts = { ef_malformed, ef_unknown }

-- Dissect exactly one PDU. `pdu_range` covers the line plus its trailing CRLF;
-- `line` is the Lua string of the line WITHOUT the CRLF. Field offsets are
-- computed relative to `base`, the PDU's offset inside the tvb.
local function dissect_pdu(tvb, pinfo, root, base, pdu_len, line)
    local subtree = root:add(xscp, tvb(base, pdu_len))

    local sep1 = line:find("|", 1, true)
    if not sep1 then
        subtree:add_proto_expert_info(ef_malformed, "PDU has no '|' delimiter")
        pinfo.cols.info:append(" [malformed]")
        return
    end

    local first    = line:sub(1, sep1 - 1)
    local first_len = sep1 - 1

    -- Response: first field is a numeric status code, single delimiter.
    if first:match("^%d+$") then
        subtree:set_text("XSCP Response")
        subtree:add(f_kind, tvb(base, pdu_len), "Response"):set_generated()
        subtree:add(f_status, tvb(base, first_len), tonumber(first))

        local reason     = line:sub(sep1 + 1)
        local reason_off  = base + sep1            -- byte right after the '|'
        local reason_len  = #line - sep1
        subtree:add(f_reason, tvb(reason_off, reason_len), reason)

        pinfo.cols.info:append(string.format(" Response: %s %s", first, reason))
        return
    end

    -- Request / Notification: OPCODE|Source|Message, two significant delimiters.
    local kind, opcode_field
    if first == "LOGN" or first == "SEND" or first == "EXIT" then
        kind, opcode_field = "Request", f_opcode
    elseif first == "BRDC" then
        kind, opcode_field = "Notification", f_ntype
    else
        subtree:add_proto_expert_info(ef_unknown,
            "Unrecognized first field: '" .. first .. "'")
        pinfo.cols.info:append(string.format(" [unknown: %s]", first))
        return
    end

    local sep2 = line:find("|", sep1 + 1, true)
    if not sep2 then
        subtree:add_proto_expert_info(ef_malformed, "Missing second '|' delimiter")
        pinfo.cols.info:append(" [malformed]")
        return
    end

    subtree:set_text("XSCP " .. kind)
    subtree:add(f_kind, tvb(base, pdu_len), kind):set_generated()
    subtree:add(opcode_field, tvb(base, first_len), first)

    local source     = line:sub(sep1 + 1, sep2 - 1)
    local source_off  = base + sep1
    local source_len  = sep2 - sep1 - 1
    subtree:add(f_source, tvb(source_off, source_len), source)

    local message     = line:sub(sep2 + 1)
    local message_off  = base + sep2
    local message_len  = #line - sep2
    subtree:add(f_message, tvb(message_off, message_len), message)

    if message == "" then
        -- e.g. LOGN/EXIT carry no message; skip the dangling "-> ".
        pinfo.cols.info:append(string.format(" %s: %s %s", kind, first, source))
    else
        pinfo.cols.info:append(string.format(" %s: %s %s -> %s",
            kind, first, source, message))
    end
end

function xscp.dissector(tvb, pinfo, root)
    local buf_len = tvb:len()
    if buf_len == 0 then return 0 end

    pinfo.cols.protocol = xscp.name
    pinfo.cols.info:clear()

    local offset = 0
    while offset < buf_len do
        local data = tvb:raw(offset)            -- remaining bytes as a Lua string
        local crlf = data:find("\r\n", 1, true) -- position of '\r' (1-based)

        if not crlf then
            -- Incomplete line: ask TCP for more bytes and re-dissect from here.
            pinfo.desegment_offset = offset
            pinfo.desegment_len = DESEGMENT_ONE_MORE_SEGMENT
            return buf_len
        end

        local pdu_len = crlf + 1                 -- line bytes (crlf-1) + CRLF (2)
        local line    = data:sub(1, crlf - 1)    -- the line without CRLF
        dissect_pdu(tvb, pinfo, root, offset, pdu_len, line)

        offset = offset + pdu_len
    end

    return buf_len
end

DissectorTable.get("tcp.port"):add(DEFAULT_PORT, xscp)
