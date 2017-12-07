use std::slice;

use goblin::pe;

use {Machine, Object, ObjectSection, ObjectSegment, SectionKind, Symbol};

/// A PE object file.
#[derive(Debug)]
pub struct PeFile<'data> {
    pe: pe::PE<'data>,
    data: &'data [u8],
}

/// An iterator over the loadable sections of a `PeFile`.
#[derive(Debug)]
pub struct PeSegmentIterator<'data, 'file>
where
    'data: 'file,
{
    file: &'file PeFile<'data>,
    iter: slice::Iter<'file, pe::section_table::SectionTable>,
}

/// A loadable section of a `PeFile`.
#[derive(Debug)]
pub struct PeSegment<'data, 'file>
where
    'data: 'file,
{
    file: &'file PeFile<'data>,
    section: &'file pe::section_table::SectionTable,
}

/// An iterator over the sections of a `PeFile`.
#[derive(Debug)]
pub struct PeSectionIterator<'data, 'file>
where
    'data: 'file,
{
    file: &'file PeFile<'data>,
    iter: slice::Iter<'file, pe::section_table::SectionTable>,
}

/// A section of a `PeFile`.
#[derive(Debug)]
pub struct PeSection<'data, 'file>
where
    'data: 'file,
{
    file: &'file PeFile<'data>,
    section: &'file pe::section_table::SectionTable,
}

impl<'data> PeFile<'data> {
    /// Get the PE headers of the file.
    // TODO: this is temporary to allow access to features this crate doesn't provide yet
    #[inline]
    pub fn pe(&self) -> &pe::PE<'data> {
        &self.pe
    }

    /// Parse the raw PE file data.
    pub fn parse(data: &'data [u8]) -> Result<Self, &'static str> {
        let pe = pe::PE::parse(data).map_err(|_| "Could not parse PE header")?;
        Ok(PeFile { pe, data })
    }
}

impl<'data, 'file> Object<'data, 'file> for PeFile<'data>
where
    'data: 'file,
{
    type Segment = PeSegment<'data, 'file>;
    type SegmentIterator = PeSegmentIterator<'data, 'file>;
    type Section = PeSection<'data, 'file>;
    type SectionIterator = PeSectionIterator<'data, 'file>;

    fn machine(&self) -> Machine {
        match self.pe.header.coff_header.machine {
            // TODO: Arm/Arm64
            pe::header::COFF_MACHINE_X86 => Machine::X86,
            pe::header::COFF_MACHINE_X86_64 => Machine::X86_64,
            _ => Machine::Other,
        }
    }

    fn segments(&'file self) -> PeSegmentIterator<'data, 'file> {
        PeSegmentIterator {
            file: self,
            iter: self.pe.sections.iter(),
        }
    }

    fn section_data_by_name(&self, section_name: &str) -> Option<&'data [u8]> {
        for section in &self.pe.sections {
            if let Ok(name) = section.name() {
                if name == section_name {
                    return Some(
                        &self.data[section.pointer_to_raw_data as usize..]
                            [..section.size_of_raw_data as usize],
                    );
                }
            }
        }
        None
    }

    fn sections(&'file self) -> PeSectionIterator<'data, 'file> {
        PeSectionIterator {
            file: self,
            iter: self.pe.sections.iter(),
        }
    }

    fn symbols(&self) -> Vec<Symbol<'data>> {
        // TODO
        Vec::new()
    }

    #[inline]
    fn is_little_endian(&self) -> bool {
        // TODO: always little endian?  The COFF header has some bits in the
        // characteristics flags, but these are obsolete.
        true
    }
}

impl<'data, 'file> Iterator for PeSegmentIterator<'data, 'file> {
    type Item = PeSegment<'data, 'file>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|section| {
            PeSegment {
                file: self.file,
                section,
            }
        })
    }
}

impl<'data, 'file> ObjectSegment<'data> for PeSegment<'data, 'file> {
    #[inline]
    fn address(&self) -> u64 {
        u64::from(self.section.virtual_address)
    }

    #[inline]
    fn size(&self) -> u64 {
        u64::from(self.section.virtual_size)
    }

    fn data(&self) -> &'data [u8] {
        &self.file.data[self.section.pointer_to_raw_data as usize..]
            [..self.section.size_of_raw_data as usize]
    }

    #[inline]
    fn name(&self) -> Option<&str> {
        self.section.name().ok()
    }
}

impl<'data, 'file> Iterator for PeSectionIterator<'data, 'file> {
    type Item = PeSection<'data, 'file>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|section| {
            PeSection {
                file: self.file,
                section,
            }
        })
    }
}

impl<'data, 'file> ObjectSection<'data> for PeSection<'data, 'file> {
    #[inline]
    fn address(&self) -> u64 {
        u64::from(self.section.virtual_address)
    }

    #[inline]
    fn size(&self) -> u64 {
        u64::from(self.section.virtual_size)
    }

    fn data(&self) -> &'data [u8] {
        &self.file.data[self.section.pointer_to_raw_data as usize..]
            [..self.section.size_of_raw_data as usize]
    }

    fn name(&self) -> Option<&str> {
        self.section.name().ok()
    }

    #[inline]
    fn segment_name(&self) -> Option<&str> {
        None
    }

    #[inline]
    fn kind(&self) -> SectionKind {
        if self.section.characteristics
            & (pe::section_table::IMAGE_SCN_CNT_CODE | pe::section_table::IMAGE_SCN_MEM_EXECUTE)
            != 0
        {
            SectionKind::Text
        } else if self.section.characteristics & pe::section_table::IMAGE_SCN_CNT_INITIALIZED_DATA
            != 0
        {
            SectionKind::Data
        } else if self.section.characteristics & pe::section_table::IMAGE_SCN_CNT_UNINITIALIZED_DATA
            != 0
        {
            SectionKind::UninitializedData
        } else {
            SectionKind::Unknown
        }
    }
}
