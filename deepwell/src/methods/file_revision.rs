/*
 * methods/file_revision.rs
 *
 * DEEPWELL - Wikijump API provider and database manager
 * Copyright (C) 2019-2023 Wikijump Team
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
use crate::services::file::GetFile;
use crate::services::file_revision::{GetFileRevision, UpdateFileRevision};
use crate::services::revision::RevisionCountOutput;
use crate::web::FileLimitQuery;

pub async fn file_revision_count(mut req: ApiRequest) -> ApiResponse {
    let txn = req.database().begin().await?;
    let ctx = ServiceContext::new(&req, &txn);

    let GetFile {
        site_id,
        page: page_reference,
        file: file_reference,
    } = req.body_json().await?;

    tide::log::info!(
        "Getting latest revision for file {page_reference:?} in site ID {site_id}",
    );

    let file = FileService::get(&ctx, site_id, file_reference)
        .await
        .to_api()?;

    let revision_count = FileRevisionService::count(&ctx, file.page_id, file.file_id)
        .await
        .to_api()?;

    txn.commit().await?;
    let output = RevisionCountOutput {
        revision_count,
        first_revision: 0,
        last_revision: revision_count.get() - 1,
    };

    let body = Body::from_json(&output)?;
    let response = Response::builder(StatusCode::Ok).body(body).into();
    Ok(response)
}

pub async fn file_revision_get(mut req: ApiRequest) -> ApiResponse {
    let txn = req.database().begin().await?;
    let ctx = ServiceContext::new(&req, &txn);

    let input: GetFileRevision = req.body_json().await?;

    let GetFileRevision {
        page_id,
        file_id,
        revision_number,
    } = req.body_json().await?;

    tide::log::info!(
        "Getting file revision {revision_number} for file ID {file_id} on page ID {page_id}",
    );

    let revision = FileRevisionService::get(&ctx, page_id, file_id, revision_number)
        .await
        .to_api()?;

    txn.commit().await?;
    let body = Body::from_json(&revision)?;
    let response = Response::builder(StatusCode::Ok).body(body).into();
    Ok(response)
}

pub async fn file_revision_put(mut req: ApiRequest) -> ApiResponse {
    let txn = req.database().begin().await?;
    let ctx = ServiceContext::new(&req, &txn);

    let input: UpdateFileRevision = req.body_json().await?;

    tide::log::info!(
        "Editing file revision ID {} for file ID {} on page {}",
        input.revision_id,
        input.file_id,
        input.page_id,
    );

    FileRevisionService::update(&ctx, input).await.to_api()?;

    txn.commit().await?;
    Ok(Response::new(StatusCode::NoContent))
}

pub async fn file_revision_range_get(req: ApiRequest) -> ApiResponse {
    let txn = req.database().begin().await?;
    let ctx = ServiceContext::new(&req, &txn);

    let FileLimitQuery { limit } = req.query()?;

    let site_id = req.param("site_id")?.parse()?;
    let revision_number = req.param("revision_number")?.parse()?;
    let direction = req.param("direction")?.parse()?;
    let page_reference = Reference::try_from_fields_key(&req, "page_type", "id_or_slug")?;
    let file_reference = Reference::try_from_fields_key(&req, "file_type", "id_or_name")?;

    let page = PageService::get(&ctx, site_id, page_reference)
        .await
        .to_api()?;
    let file = FileService::get(&ctx, page.page_id, file_reference)
        .await
        .to_api()?;
    let revisions = FileRevisionService::get_range(
        &ctx,
        page.page_id,
        file.file_id,
        revision_number,
        direction,
        limit.into(),
    )
    .await
    .to_api()?;

    txn.commit().await?;
    let body = Body::from_json(&revisions)?;
    let response = Response::builder(StatusCode::Ok).body(body).into();
    Ok(response)
}
