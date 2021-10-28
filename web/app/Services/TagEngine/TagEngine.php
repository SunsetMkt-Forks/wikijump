<?php
declare(strict_types=1);

namespace Wikijump\Services\TagEngine;

use Ds\Set;

/**
 * The Tag Engine receives the information of the site's tagging configuration and processes new page tags.
 */

final class TagEngine
{
    private function __construct()
    {
    }

    /**
     * Verifies whether the given tags are valid or not under the passed tag configuration.
     *
     * @param TagConfiguration $config The configuration to check the tags against
     * @param Set $previous_tags The previous tags (if any) associated with this page
     * @param Set $current_tags The tags being proposed for this page
     * @param Set $role_ids The roles that the current user performing the tag action has
     * @return TagDecision The outcome of validation
     */
    public static function validate(
        TagConfiguration $config,
        Set $previous_tags,
        Set $current_tags,
        Set $role_ids
    ): TagDecision {
        // Derived parameters
        $added_tags = $current_tags->diff($previous_tags);
        $removed_tags = $previous_tags->diff($current_tags);
        $now = Carbon::now();

        // State values
        $valid = true;

        // Perform checks
        $invalid_tags = $config->validateTags($added_tags, $removed_tags, $role_ids, $now);
        $valid &= empty(result);

        [
            'tags' => $failed_tag_conditions,
            'tag_groups' => $failed_tag_group_conditions,
        ] = $config->validateConditions($current_tags);
        $valid &= empty($failed_tag_conditions) && empty($failed_tag_group_conditions);

        // Build final TagDecision
        return new TagDecision($valid, $invalid_tags, $failed_tag_conditions, $failed_tag_group_conditions);
    }
}
