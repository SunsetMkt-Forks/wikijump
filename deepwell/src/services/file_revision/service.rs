/*
 * services/file_revision/service.rs
 *
 * DEEPWELL - Wikijump API provider and database manager
 * Copyright (C) 2019-2022 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

use super::prelude::*;
use crate::json_utils::string_list_to_json;
use crate::models::file_revision::{
    self, Entity as FileRevision, Model as FileRevisionModel,
};
use crate::services::{OutdateService, PageService};
use crate::web::FetchDirection;
use serde_json::json;
use std::num::NonZeroI32;

#[derive(Debug)]
pub struct FileRevisionService;

impl FileRevisionService {
    /// Creates a new revision on an existing file.
    ///
    /// See `RevisionService::create()`.
    ///
    /// # Panics
    /// If the given previous revision is for a different file or page, this method will panic.
    pub async fn create(
        ctx: &ServiceContext<'_>,
        CreateFileRevision {
            site_id,
            mut page_id,
            file_id,
            user_id,
            comments,
            body,
        }: CreateFileRevision,
        previous: FileRevisionModel,
    ) -> Result<Option<CreateFileRevisionOutput>> {
        let txn = ctx.transaction();
        let revision_number = next_revision_number(&previous, page_id, &file_id);

        // Fields to create in the revision
        let mut changes = Vec::new();
        let FileRevisionModel {
            mut name,
            mut s3_hash,
            mut mime_hint,
            mut size_hint,
            mut licensing,
            ..
        } = previous;

        // Update fields from input
        //
        // We check the values so that the only listed "changes"
        // are those that actually are different.

        if let ProvidedValue::Set(new_page_id) = body.page_id {
            if page_id != new_page_id {
                changes.push("page");
                page_id = new_page_id;
            }
        }

        if let ProvidedValue::Set(new_name) = body.name {
            if name != new_name {
                changes.push("name");
                name = new_name;
            }
        }

        if let ProvidedValue::Set(new_blob) = body.blob {
            if s3_hash != new_blob.s3_hash
                || size_hint != new_blob.size_hint
                || mime_hint != new_blob.mime_hint
            {
                changes.push("blob");
                s3_hash = new_blob.s3_hash.to_vec();
                size_hint = new_blob.size_hint;
                mime_hint = new_blob.mime_hint;
            }
        }

        if let ProvidedValue::Set(new_licensing) = body.licensing {
            if licensing != new_licensing {
                changes.push("licensing");
                licensing = new_licensing;
            }
        }

        // If nothing has changed, then don't create a new revision
        if changes.is_empty() {
            // TODO rerender page
            return Ok(None);
        }

        // Validate inputs
        if name.is_empty() || name.len() >= 256 {
            tide::log::error!("File name of invalid length: {}", name.len());
            return Err(Error::BadRequest);
        }

        if mime_hint.is_empty() {
            tide::log::error!("MIME type hint is empty");
            return Err(Error::BadRequest);
        }

        // TODO validate licensing field

        // Run outdater
        let page_slug = Self::get_page_slug(ctx, site_id, page_id).await?;
        OutdateService::process_page_edit(ctx, site_id, page_id, &page_slug).await?;

        // Insert the new revision into the table
        let changes = string_list_to_json(&changes)?;
        let model = file_revision::ActiveModel {
            revision_type: Set(FileRevisionType::Update),
            revision_number: Set(0),
            file_id: Set(file_id.clone()),
            page_id: Set(page_id),
            user_id: Set(user_id),
            name: Set(name),
            s3_hash: Set(s3_hash.to_vec()),
            size_hint: Set(size_hint),
            mime_hint: Set(mime_hint),
            licensing: Set(licensing),
            changes: Set(changes),
            comments: Set(comments),
            hidden: Set(json!([])),
            ..Default::default()
        };

        let FileRevisionModel { revision_id, .. } = model.insert(txn).await?;
        Ok(Some(CreateFileRevisionOutput {
            file_revision_id: revision_id,
            file_revision_number: revision_number,
        }))
    }

    /// Creates the first revision for a newly-uploaded file.
    ///
    /// See `RevisionService::create_first()`.
    ///
    /// # Panics
    /// If the given previous revision is for a different file or page, this method will panic.
    pub async fn create_first(
        ctx: &ServiceContext<'_>,
        CreateFirstFileRevision {
            page_id,
            site_id,
            file_id,
            user_id,
            name,
            s3_hash,
            size_hint,
            mime_hint,
            licensing,
            comments,
        }: CreateFirstFileRevision,
    ) -> Result<CreateFirstFileRevisionOutput> {
        let txn = ctx.transaction();

        // Run outdater
        let page_slug = Self::get_page_slug(ctx, site_id, page_id).await?;
        OutdateService::process_page_displace(ctx, site_id, page_id, &page_slug).await?;

        // Effective constant, number of changes for the first revision.
        // The first revision is always considered to have changed everything.
        let all_changes = json!(["page", "name", "blob", "mime", "licensing"]);

        // Insert the first revision into the table
        let model = file_revision::ActiveModel {
            revision_type: Set(FileRevisionType::Create),
            revision_number: Set(0),
            file_id: Set(file_id.clone()),
            page_id: Set(page_id),
            user_id: Set(user_id),
            name: Set(name),
            s3_hash: Set(s3_hash.to_vec()),
            mime_hint: Set(mime_hint),
            size_hint: Set(size_hint),
            licensing: Set(licensing),
            changes: Set(all_changes),
            comments: Set(comments),
            hidden: Set(json!([])),
            ..Default::default()
        };

        let FileRevisionModel { revision_id, .. } = model.insert(txn).await?;
        Ok(CreateFirstFileRevisionOutput {
            file_id,
            file_revision_id: revision_id,
        })
    }

    /// Creates a revision marking a page as deleted.
    ///
    /// This revision is called a "tombstone" in that
    /// its only purpose is to mark that the file has been deleted.
    ///
    /// See `RevisionService::create_tombstone()`.
    ///
    /// # Panics
    /// If the given previous revision is for a different file or page, this method will panic.
    pub async fn create_tombstone(
        ctx: &ServiceContext<'_>,
        CreateTombstoneFileRevision {
            site_id,
            page_id,
            file_id,
            user_id,
            comments,
        }: CreateTombstoneFileRevision,
        previous: FileRevisionModel,
    ) -> Result<CreateFileRevisionOutput> {
        let txn = ctx.transaction();
        let revision_number = next_revision_number(&previous, page_id, &file_id);

        let FileRevisionModel {
            name,
            s3_hash,
            mime_hint,
            size_hint,
            licensing,
            ..
        } = previous;

        // Run outdater
        let page_slug = Self::get_page_slug(ctx, site_id, page_id).await?;
        OutdateService::process_page_edit(ctx, site_id, page_id, &page_slug).await?;

        // Insert the tombstone revision into the table
        let model = file_revision::ActiveModel {
            revision_type: Set(FileRevisionType::Delete),
            revision_number: Set(revision_number),
            file_id: Set(file_id.clone()),
            page_id: Set(page_id),
            user_id: Set(user_id),
            name: Set(name),
            s3_hash: Set(s3_hash),
            mime_hint: Set(mime_hint),
            size_hint: Set(size_hint),
            licensing: Set(licensing),
            changes: Set(json!([])),
            comments: Set(comments),
            hidden: Set(json!([])),
            ..Default::default()
        };

        let FileRevisionModel { revision_id, .. } = model.insert(txn).await?;
        Ok(CreateFileRevisionOutput {
            file_revision_id: revision_id,
            file_revision_number: revision_number,
        })
    }

    /// Creates a revision marking a pages as restored (i.e., undeleted).
    ///
    /// Similar to `create_tombstone`, this method creates
    /// a revision whose only purpose is to mark that the page
    /// has been restored.
    ///
    /// Note that page parenting information is removed during deletion
    /// and is not restored here.
    ///
    /// Remember that, like `create_first()`, this method assumes
    /// the caller has already verified that undeleting the page here
    /// will not cause conflicts.
    ///
    /// See `RevisionService::create_tombstone()`.
    ///
    /// # Panics
    /// If the given previous revision is for a different file or page, this method will panic.
    pub async fn create_resurrection(
        ctx: &ServiceContext<'_>,
        CreateResurrectionFileRevision {
            site_id,
            page_id: old_page_id,
            file_id,
            user_id,
            new_page_id,
            new_name,
            comments,
        }: CreateResurrectionFileRevision,
        previous: FileRevisionModel,
    ) -> Result<CreateFileRevisionOutput> {
        let txn = ctx.transaction();
        let revision_number = next_revision_number(&previous, old_page_id, &file_id);

        let FileRevisionModel {
            name: old_name,
            s3_hash,
            mime_hint,
            size_hint,
            licensing,
            ..
        } = previous;

        let changes = {
            let mut changes = vec![];

            if old_page_id != new_page_id {
                changes.push("page");
            }

            if old_name != new_name {
                changes.push("name");
            }

            changes
        };

        // Run outdater
        let new_page_slug = Self::get_page_slug(ctx, site_id, new_page_id).await?;
        OutdateService::process_page_edit(ctx, site_id, new_page_id, &new_page_slug)
            .await?;

        // Insert the resurrection revision into the table
        let changes = string_list_to_json(&changes)?;
        let model = file_revision::ActiveModel {
            revision_type: Set(FileRevisionType::Undelete),
            revision_number: Set(revision_number),
            file_id: Set(file_id.clone()),
            page_id: Set(new_page_id),
            user_id: Set(user_id),
            name: Set(new_name),
            s3_hash: Set(s3_hash),
            mime_hint: Set(mime_hint),
            size_hint: Set(size_hint),
            licensing: Set(licensing),
            changes: Set(changes),
            comments: Set(comments),
            hidden: Set(json!([])),
            ..Default::default()
        };

        let FileRevisionModel { revision_id, .. } = model.insert(txn).await?;
        Ok(CreateFileRevisionOutput {
            file_revision_id: revision_id,
            file_revision_number: revision_number,
        })
    }

    /// Modifies an existing file revision.
    ///
    /// Revisions are immutable entries in an append-only log.
    /// However, the `hidden` column can be updated to "delete"
    /// revisions (wholly or partially) to cover spam and abuse.
    pub async fn update(
        ctx: &ServiceContext<'_>,
        page_id: i64,
        file_id: &str,
        revision_id: i64,
        UpdateFileRevision { user_id, hidden }: UpdateFileRevision,
    ) -> Result<()> {
        let txn = ctx.transaction();

        // The latest file revision cannot be hidden, because
        // the file, its name, contents, etc are exposed.
        // It should be reverted first, and then it can be hidden.

        let latest = Self::get_latest(ctx, page_id, file_id).await?;
        if revision_id == latest.revision_id {
            return Err(Error::CannotHideLatestRevision);
        }

        // TODO: record revision edit in audit log
        let _ = user_id;

        // Update the revision

        let hidden = string_list_to_json(&hidden)?;
        let model = file_revision::ActiveModel {
            revision_id: Set(revision_id),
            hidden: Set(hidden),
            ..Default::default()
        };

        // Update and return
        model.update(txn).await?;
        Ok(())
    }

    /// Get the latest revision for this file.
    ///
    /// See `RevisionService::get_latest()`.
    pub async fn get_latest(
        ctx: &ServiceContext<'_>,
        page_id: i64,
        file_id: &str,
    ) -> Result<FileRevisionModel> {
        // NOTE: There is no optional variant of this method,
        //       since all extant files must have at least one revision.

        let txn = ctx.transaction();
        let revision = FileRevision::find()
            .filter(
                Condition::all()
                    .add(file_revision::Column::PageId.eq(page_id))
                    .add(file_revision::Column::FileId.eq(file_id)),
            )
            .order_by_desc(file_revision::Column::RevisionNumber)
            .one(txn)
            .await?
            .ok_or(Error::NotFound)?;

        Ok(revision)
    }

    /// Get the given revision for a file.
    ///
    /// See `RevisionService::get_optional()`.
    pub async fn get_optional(
        ctx: &ServiceContext<'_>,
        page_id: i64,
        file_id: &str,
        revision_number: i32,
    ) -> Result<Option<FileRevisionModel>> {
        let txn = ctx.transaction();
        let revision = FileRevision::find()
            .filter(
                Condition::all()
                    .add(file_revision::Column::PageId.eq(page_id))
                    .add(file_revision::Column::FileId.eq(file_id))
                    .add(file_revision::Column::RevisionNumber.eq(revision_number)),
            )
            .one(txn)
            .await?;

        Ok(revision)
    }

    /// Determines if the given file revision exists.
    ///
    /// See `RevisionService::exists()`.
    #[inline]
    pub async fn exists(
        ctx: &ServiceContext<'_>,
        page_id: i64,
        file_id: &str,
        revision_number: i32,
    ) -> Result<bool> {
        Self::get_optional(ctx, page_id, file_id, revision_number)
            .await
            .map(|revision| revision.is_some())
    }

    /// Gets the given revision for a file, failing if it doesn't exist.
    ///
    /// See `RevisionService::get()`.
    #[inline]
    pub async fn get(
        ctx: &ServiceContext<'_>,
        page_id: i64,
        file_id: &str,
        revision_number: i32,
    ) -> Result<FileRevisionModel> {
        Self::get_optional(ctx, page_id, file_id, revision_number)
            .await?
            .ok_or(Error::NotFound)
    }

    /// Counts the number of revisions for a file.
    ///
    /// See `RevisionService::count()`.
    pub async fn count(
        ctx: &ServiceContext<'_>,
        page_id: i64,
        file_id: &str,
    ) -> Result<NonZeroI32> {
        let txn = ctx.transaction();
        let row_count = FileRevision::find()
            .filter(
                Condition::all()
                    .add(file_revision::Column::PageId.eq(page_id))
                    .add(file_revision::Column::FileId.eq(file_id)),
            )
            .count(txn)
            .await?;

        // We store revision_number in INT, which is i32.
        // So even though this row count is usize, it
        // should always fit inside an i32.
        let row_count = i32::try_from(row_count)
            .expect("Revision row count greater than revision_number integer size");

        // All pages have at least one revision, so if there are none
        // that means this page does not exist, and we should return an error.
        match NonZeroI32::new(row_count) {
            Some(count) => Ok(count),
            None => Err(Error::NotFound),
        }
    }

    /// Gets a range of revisions for a file.
    ///
    /// See `RevisionService::get_range()`.
    pub async fn get_range(
        ctx: &ServiceContext<'_>,
        page_id: i64,
        file_id: &str,
        revision_number: i32,
        revision_direction: FetchDirection,
        revision_limit: u64,
    ) -> Result<Vec<FileRevisionModel>> {
        let revision_condition = {
            use file_revision::Column::RevisionNumber;

            // Allow specifying "-1" to mean "the most recent revision",
            // otherwise keep as-is.
            let revision_number = if revision_number >= 0 {
                revision_number
            } else {
                i32::MAX
            };

            // Get correct database condition based on requested ordering
            match revision_direction {
                FetchDirection::Before => RevisionNumber.lte(revision_number),
                FetchDirection::After => RevisionNumber.gte(revision_number),
            }
        };

        let txn = ctx.transaction();
        let revisions = FileRevision::find()
            .filter(
                Condition::all()
                    .add(file_revision::Column::PageId.eq(page_id))
                    .add(file_revision::Column::FileId.eq(file_id))
                    .add(revision_condition),
            )
            .order_by_asc(file_revision::Column::RevisionNumber)
            .limit(revision_limit)
            .all(txn)
            .await?;

        Ok(revisions)
    }

    async fn get_page_slug(
        ctx: &ServiceContext<'_>,
        site_id: i64,
        page_id: i64,
    ) -> Result<String> {
        let page = PageService::get(ctx, site_id, Reference::Id(page_id)).await?;
        Ok(page.slug)
    }
}

fn next_revision_number(
    previous: &FileRevisionModel,
    page_id: i64,
    file_id: &str,
) -> i32 {
    // Check for basic consistency
    assert_eq!(
        previous.file_id, file_id,
        "Previous revision has an inconsistent file ID",
    );
    assert_eq!(
        previous.page_id, page_id,
        "Previous revision has an inconsistent page ID",
    );

    // Get the new revision number
    previous.revision_number + 1
}