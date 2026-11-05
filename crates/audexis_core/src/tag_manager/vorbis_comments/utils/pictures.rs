use super::*;

pub fn parse_picture(buf: &[u8]) -> Result<TagValue, Error> {
    let mut offset = 0;

    let read_u32 = |buf: &[u8], offset: &mut usize| -> Result<u32, Error> {
        if *offset + 4 > buf.len() {
            return Err(Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Unexpected EOF while reading picture u32",
            ));
        }
        let bytes: [u8; 4] = buf[*offset..*offset + 4].try_into().unwrap();
        *offset += 4;
        Ok(u32::from_be_bytes(bytes))
    };

    let picture_type = read_u32(buf, &mut offset)?;

    let mime_len = read_u32(buf, &mut offset)? as usize;
    if offset + mime_len > buf.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading picture mime",
        ));
    }
    let mime = String::from_utf8(buf[offset..offset + mime_len].to_vec());
    if mime.is_err() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Picture mime is not valid UTF-8",
        ));
    }
    let mime = mime.unwrap();
    offset += mime_len;

    let desc_len = read_u32(buf, &mut offset)? as usize;
    if offset + desc_len > buf.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading picture description",
        ));
    }
    let description = String::from_utf8(buf[offset..offset + desc_len].to_vec());
    if description.is_err() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Picture description is not valid UTF-8",
        ));
    }
    let description = description.unwrap();
    offset += desc_len;

    // useless
    let _width = read_u32(buf, &mut offset)?;
    let _height = read_u32(buf, &mut offset)?;
    let _depth = read_u32(buf, &mut offset)?;
    let _indexed_colors = read_u32(buf, &mut offset)?;

    let data_len = read_u32(buf, &mut offset)? as usize;
    if offset + data_len > buf.len() {
        return Err(Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Unexpected EOF while reading picture data",
        ));
    }
    let data = buf[offset..offset + data_len].to_vec();
    let pt = u8::try_from(picture_type);
    if pt.is_err() {
        return Err(Error::new(
            std::io::ErrorKind::InvalidData,
            "Picture type out of range",
        ));
    }
    let picture_type = pt.unwrap();

    Ok(TagValue::Picture {
        picture_type: Some(picture_type),
        mime,
        description: Some(description),
        data,
    })
}

pub fn build_picture_comment(tag: &TagValue) -> Option<Vec<u8>> {
    if let TagValue::Picture {
        picture_type,
        mime,
        description,
        data,
    } = tag
    {
        let mut out = Vec::new();

        out.extend(&(picture_type.unwrap_or(3) as u32).to_be_bytes());

        let mime_bytes = mime.as_bytes();
        out.extend(&(mime_bytes.len() as u32).to_be_bytes());
        out.extend(mime_bytes);

        let desc_bytes = description.as_deref().unwrap_or("").as_bytes();
        out.extend(&(desc_bytes.len() as u32).to_be_bytes());
        out.extend(desc_bytes);

        out.extend(&0u32.to_be_bytes());
        out.extend(&0u32.to_be_bytes());
        out.extend(&0u32.to_be_bytes());
        out.extend(&0u32.to_be_bytes());

        out.extend(&(data.len() as u32).to_be_bytes());
        out.extend(data);

        Some(out)
    } else {
        None
    }
}

pub fn build_picture_tag(tags: &HashMap<FrameKey, Vec<TagValue>>) -> Vec<Vec<u8>> {
    let mut pictures = Vec::new();

    for values in tags.values() {
        for value in values {
            if let TagValue::Picture {
                picture_type,
                mime,
                description,
                data,
            } = value
            {
                let mut out = Vec::new();

                out.extend(&(picture_type.unwrap_or(3) as u32).to_be_bytes());

                let mime_bytes = mime.as_bytes();
                out.extend(&(mime_bytes.len() as u32).to_be_bytes());
                out.extend(mime_bytes);

                let desc_bytes = description.as_deref().unwrap_or("").as_bytes();
                out.extend(&(desc_bytes.len() as u32).to_be_bytes());
                out.extend(desc_bytes);

                out.extend(&0u32.to_be_bytes());
                out.extend(&0u32.to_be_bytes());
                out.extend(&0u32.to_be_bytes());
                out.extend(&0u32.to_be_bytes());

                out.extend(&(data.len() as u32).to_be_bytes());
                out.extend(data);

                pictures.push(out);
            }
        }
    }

    pictures
}
